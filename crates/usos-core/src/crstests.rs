//! The user's course tests, "sprawdziany" (`services/crstests/*`): a tree of folders, tasks (points)
//! and grade nodes per course edition.
//!
//! [`Client::course_tests`] needs one request for the list of tests (`participant2`, which also
//! carries the course edition and the root node) and, per test, one for the tree shape plus one per
//! node:
//!
//! - `node2` on the root with `subnodes_deep[id|type]` gives the shape. PolSl ignores every other
//!   inner field of `subnodes_deep` (and `node2` rejects secondary fields inside `subnodes`), so
//!   the shape holds ids only.
//! - `node2` on each node gives its name, order, visibility and, by type, `task_node_details`
//!   (`points_max` and the user's points with `last_changed` and the comment), `folder_node_details`
//!   (`points_max`) and `grade_node_details` (the user's grade). Nodes are fetched concurrently.
//!
//! The multi-node `user_points` would save the points lookups, but nothing batches the maxima, so
//! the per-node call already has to be made.
//!
//! USOS does not total points for folders and the root in the student view (folders only report
//! `points_max`), so [`sum_points`] and [`sum_max`] do it; the totals are stored in the folder and
//! root nodes. Automatic grading is not executed by USOS: only points and grades set by lecturers
//! are returned.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDateTime;
use futures_util::StreamExt;
use futures_util::stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::lenient::{lenient, lenient_or_default, opt_datetime, opt_f64, opt_string};
use crate::models::LangDict;

/// What `participant2` returns per test: the course edition, the group restriction and the root.
const PARTICIPANT_FIELDS: &str =
    "course_edition[course[id|name]|term[id]]|is_limited_to_groups|root[id|name|description]";

/// The tree shape of one test.
const SHAPE_FIELDS: &str = "id|type|subnodes_deep[id|type]";

/// Everything shown about one node; the details that do not fit the node's type come back `null`.
const NODE_FIELDS: &str = "id|type|order|name|description|results_visible_to_students|\
task_node_details[points_min|points_max|points_precision|students_points[points|comment|last_changed]]|\
folder_node_details[points_min|points_max]|\
grade_node_details[grade_type|students_grade[grade_value[symbol|passes|decimal_value|name]|comment|last_changed]]";

/// How many node requests run at the same time.
const CONCURRENCY: usize = 8;

/// The node types of a test.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Root,
    /// Groups other nodes; its points are the sum of its subtree.
    Folder,
    /// Gives points.
    Task,
    /// Gives a grade.
    Grade,
    /// A type this client does not know.
    #[default]
    #[serde(other)]
    Unknown,
}

impl NodeKind {
    fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "root" => Self::Root,
            "folder" => Self::Folder,
            "task" => Self::Task,
            "grade" => Self::Grade,
            _ => Self::Unknown,
        }
    }
}

/// The user's grade in a grade node.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeGrade {
    /// Symbol as shown to students: `4,5`, `ZAL`, ...
    pub symbol: Option<String>,
    pub description: LangDict,
    pub passes: Option<bool>,
    /// The numeric value, when the grade has one.
    pub value: Option<f64>,
}

/// One node of a test tree.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TestNode {
    pub id: String,
    pub kind: NodeKind,
    pub name: LangDict,
    pub description: LangDict,
    /// Whether the lecturer made the results of this node visible to students.
    pub results_visible: Option<bool>,
    /// The lecturer hid the results of this node (`results_visible == Some(false)`). A hidden node
    /// stays in the tree but is left out of every total.
    pub hidden: bool,
    /// The user's points: for a task what the lecturer gave, for a folder or the root the
    /// [`sum_points`] of its children; `None` when there is nothing to sum.
    pub points: Option<f64>,
    /// The most points the node gives: from USOS for tasks and folders, the [`sum_max`] of the
    /// children for the root, and for a folder with a hidden node below it (USOS counts the
    /// hidden maxima, which the student cannot reach credit for seeing).
    pub points_max: Option<f64>,
    pub points_min: Option<f64>,
    /// The user's grade (grade nodes only, and only when one was given).
    pub grade: Option<NodeGrade>,
    /// The lecturer's comment on the points or the grade.
    pub comment: Option<String>,
    /// When the points or grade were last changed; for a folder or the root the latest change below.
    pub last_changed: Option<NaiveDateTime>,
    /// Sorted by the lecturer's order.
    pub children: Vec<TestNode>,
}

/// The test of one course edition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestRoot {
    pub course_id: String,
    pub course_name: LangDict,
    pub term_id: String,
    /// The test is only for some class groups.
    pub limited_to_groups: bool,
    /// The root node; its `children` are the first level of the test.
    pub root: TestNode,
}

/// The result of [`Client::course_tests`]: the tests that loaded, and the ids of the root nodes
/// of those that could not be loaded.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CourseTests {
    pub tests: Vec<TestRoot>,
    /// Root ids of the tests skipped because USOS refused their shape (an API error other than
    /// authorization) or returned one that could not be read.
    pub skipped: Vec<String>,
}

/// The sum of the points of the visible `nodes` (a folder counts with its own total), or `None`
/// when none of them has points.
pub fn sum_points(nodes: &[TestNode]) -> Option<f64> {
    sum(nodes.iter().filter(|n| !n.hidden).filter_map(|n| n.points))
}

/// The sum of the maxima of the visible `nodes`, or `None` when none of them has one.
pub fn sum_max(nodes: &[TestNode]) -> Option<f64> {
    sum(nodes
        .iter()
        .filter(|n| !n.hidden)
        .filter_map(|n| n.points_max))
}

/// Whether any node below `node` is hidden.
fn hidden_below(node: &TestNode) -> bool {
    node.children.iter().any(|c| c.hidden || hidden_below(c))
}

fn sum(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |acc, v| Some(acc.unwrap_or(0.0) + v))
}

/// Fill the totals of folders and the root from their children, bottom-up. Tasks and grade nodes
/// keep what the API gave. A folder keeps the `points_max` USOS reported, unless a node below it is
/// hidden: then the maximum is the sum of the visible children's.
fn aggregate(node: &mut TestNode) {
    for child in &mut node.children {
        aggregate(child);
    }
    if !matches!(node.kind, NodeKind::Folder | NodeKind::Root) {
        return;
    }
    node.points = sum_points(&node.children);
    node.points_max = if hidden_below(node) {
        sum_max(&node.children)
    } else {
        node.points_max.or_else(|| sum_max(&node.children))
    };
    node.last_changed = node
        .children
        .iter()
        .filter_map(|c| c.last_changed)
        .chain(node.last_changed)
        .max();
}

// ---- API shapes (private) ----

#[derive(Deserialize)]
struct RawParticipation {
    #[serde(default, deserialize_with = "lenient")]
    course_edition: Option<RawEdition>,
    #[serde(default, deserialize_with = "lenient")]
    is_limited_to_groups: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    root: Option<RawRootRef>,
}

#[derive(Deserialize)]
struct RawEdition {
    #[serde(default, deserialize_with = "lenient")]
    course: Option<RawCourse>,
    #[serde(default, deserialize_with = "lenient")]
    term: Option<RawTerm>,
}

#[derive(Deserialize)]
struct RawCourse {
    #[serde(default, deserialize_with = "opt_string")]
    id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    name: LangDict,
}

#[derive(Deserialize)]
struct RawTerm {
    #[serde(default, deserialize_with = "opt_string")]
    id: Option<String>,
}

#[derive(Deserialize)]
struct RawRootRef {
    #[serde(default, deserialize_with = "opt_string")]
    id: Option<String>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    name: LangDict,
    #[serde(default, deserialize_with = "lenient_or_default")]
    description: LangDict,
}

/// A node of the `subnodes_deep` shape: the id and the nested children.
#[derive(Deserialize)]
struct RawShape {
    #[serde(default, deserialize_with = "opt_string")]
    id: Option<String>,
    #[serde(default, alias = "subnodes", deserialize_with = "lenient_or_default")]
    subnodes_deep: Vec<RawShape>,
}

#[derive(Deserialize)]
struct RawNode {
    #[serde(default, deserialize_with = "opt_string")]
    id: Option<String>,
    #[serde(rename = "type", default, deserialize_with = "opt_string")]
    kind: Option<String>,
    #[serde(default, deserialize_with = "opt_order")]
    order: Option<i64>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    name: LangDict,
    #[serde(default, deserialize_with = "lenient_or_default")]
    description: LangDict,
    #[serde(default, deserialize_with = "lenient")]
    results_visible_to_students: Option<bool>,
    #[serde(default, deserialize_with = "lenient")]
    task_node_details: Option<RawTask>,
    #[serde(default, deserialize_with = "lenient")]
    folder_node_details: Option<RawFolder>,
    #[serde(default, deserialize_with = "lenient")]
    grade_node_details: Option<RawGradeNode>,
}

#[derive(Deserialize)]
struct RawTask {
    #[serde(default, deserialize_with = "opt_f64")]
    points_min: Option<f64>,
    #[serde(default, deserialize_with = "opt_f64")]
    points_max: Option<f64>,
    #[serde(default, deserialize_with = "lenient")]
    students_points: Option<RawPoints>,
}

#[derive(Deserialize)]
struct RawPoints {
    #[serde(default, deserialize_with = "opt_f64")]
    points: Option<f64>,
    #[serde(default, deserialize_with = "opt_string")]
    comment: Option<String>,
    #[serde(default, deserialize_with = "opt_datetime")]
    last_changed: Option<NaiveDateTime>,
}

#[derive(Deserialize)]
struct RawFolder {
    #[serde(default, deserialize_with = "opt_f64")]
    points_min: Option<f64>,
    #[serde(default, deserialize_with = "opt_f64")]
    points_max: Option<f64>,
}

#[derive(Deserialize)]
struct RawGradeNode {
    #[serde(default, deserialize_with = "lenient")]
    students_grade: Option<RawStudentGrade>,
}

#[derive(Deserialize)]
struct RawStudentGrade {
    #[serde(default, deserialize_with = "lenient")]
    grade_value: Option<RawGradeValue>,
    #[serde(default, deserialize_with = "opt_string")]
    comment: Option<String>,
    #[serde(default, deserialize_with = "opt_datetime")]
    last_changed: Option<NaiveDateTime>,
}

#[derive(Deserialize)]
struct RawGradeValue {
    #[serde(default, deserialize_with = "opt_string")]
    symbol: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    passes: Option<bool>,
    #[serde(default, deserialize_with = "opt_f64")]
    decimal_value: Option<f64>,
    #[serde(default, deserialize_with = "lenient_or_default")]
    name: LangDict,
}

fn opt_order<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Option<i64>, D::Error> {
    Ok(opt_string(d)?.and_then(|s| s.trim().parse().ok()))
}

impl RawNode {
    /// The node without children, and its sort key.
    fn into_node(self, fallback_id: &str) -> (Option<i64>, TestNode) {
        let kind = self
            .kind
            .as_deref()
            .map_or(NodeKind::Unknown, NodeKind::parse);
        let mut node = TestNode {
            id: self.id.unwrap_or_else(|| fallback_id.to_owned()),
            kind,
            name: self.name,
            description: self.description,
            results_visible: self.results_visible_to_students,
            hidden: self.results_visible_to_students == Some(false),
            ..TestNode::default()
        };
        if let Some(task) = self.task_node_details {
            node.points_min = task.points_min;
            node.points_max = task.points_max;
            if let Some(p) = task.students_points {
                node.points = p.points;
                node.comment = p.comment;
                node.last_changed = p.last_changed;
            }
        }
        if let Some(folder) = self.folder_node_details {
            node.points_min = folder.points_min;
            node.points_max = folder.points_max;
        }
        if let Some(sg) = self.grade_node_details.and_then(|g| g.students_grade) {
            node.comment = sg.comment;
            node.last_changed = sg.last_changed;
            node.grade = sg.grade_value.map(|v| NodeGrade {
                symbol: v.symbol,
                description: v.name,
                passes: v.passes,
                value: v.decimal_value,
            });
        }
        (self.order, node)
    }
}

/// The ids below the shape's root, parents before children, without duplicates.
fn shape_ids(shape: &RawShape, out: &mut Vec<String>) {
    let mut seen: HashSet<String> = out.iter().cloned().collect();
    collect_ids(shape, &mut seen, out);
}

fn collect_ids(shape: &RawShape, seen: &mut HashSet<String>, out: &mut Vec<String>) {
    for child in &shape.subnodes_deep {
        if let Some(id) = &child.id {
            if seen.insert(id.clone()) {
                out.push(id.clone());
            }
            collect_ids(child, seen, out);
        }
    }
}

/// Ids compare numerically when both are numbers, else as strings.
fn cmp_ids(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

/// The children of `shape` as nodes taken from `nodes` (a node used once; ids without details
/// are skipped), sorted by the lecturer's order, then id.
fn build_children(
    shape: &RawShape,
    nodes: &mut HashMap<String, (Option<i64>, TestNode)>,
) -> Vec<TestNode> {
    let mut children: Vec<(Option<i64>, TestNode)> = shape
        .subnodes_deep
        .iter()
        .filter_map(|s| {
            let (order, mut node) = nodes.remove(s.id.as_deref()?)?;
            node.children = build_children(s, nodes);
            Some((order, node))
        })
        .collect();
    children.sort_by(|(oa, a), (ob, b)| {
        oa.unwrap_or(i64::MAX)
            .cmp(&ob.unwrap_or(i64::MAX))
            .then_with(|| cmp_ids(&a.id, &b.id))
    });
    children.into_iter().map(|(_, n)| n).collect()
}

/// Parse a `participant2` response: the tests in `term_ids`, with an empty root node. Malformed
/// items and tests without a root, course or term are skipped. Sorted by term, then course.
fn parse_participants(value: &Value, term_ids: &[&str]) -> Result<Vec<TestRoot>> {
    let Value::Array(items) = value else {
        return Err(Error::UnexpectedResponse(
            "unexpected tests shape (not an array)".to_owned(),
        ));
    };
    let mut out: Vec<TestRoot> = items
        .iter()
        .filter_map(|item| serde_json::from_value::<RawParticipation>(item.clone()).ok())
        .filter_map(|raw| {
            let edition = raw.course_edition?;
            let course = edition.course?;
            let term_id = edition.term?.id?;
            let root = raw.root?;
            let root_id = root.id?;
            term_ids.contains(&term_id.as_str()).then(|| TestRoot {
                course_id: course.id.unwrap_or_default(),
                course_name: course.name,
                term_id,
                limited_to_groups: raw.is_limited_to_groups.unwrap_or(false),
                root: TestNode {
                    id: root_id,
                    kind: NodeKind::Root,
                    name: root.name,
                    description: root.description,
                    ..TestNode::default()
                },
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.term_id
            .cmp(&b.term_id)
            .then_with(|| cmp_ids(&a.course_id, &b.course_id))
            .then_with(|| cmp_ids(&a.root.id, &b.root.id))
    });
    Ok(out)
}

/// Put the nodes of `shape` below `root` and fill the totals.
fn assemble(root: &mut TestNode, shape: &RawShape, details: Vec<(String, Value)>) {
    let mut nodes: HashMap<String, (Option<i64>, TestNode)> = details
        .into_iter()
        .filter_map(|(id, value)| {
            let raw = serde_json::from_value::<RawNode>(value).ok()?;
            let entry = raw.into_node(&id);
            Some((id, entry))
        })
        .collect();
    root.children = build_children(shape, &mut nodes);
    aggregate(root);
}

/// An API error that ends the whole run (authorization), as opposed to one that only costs a node
/// or a test.
fn is_fatal(e: &Error) -> bool {
    match e {
        Error::Api(api) => api.is_unauthorized() || api.is_scope_missing(),
        _ => true,
    }
}

impl Client {
    /// The tests of the user's course editions in `term_ids`, sorted by term and course. Terms
    /// without tests give nothing. One request lists the tests, then each test costs one request
    /// for its shape and one per node (see the module docs); all of them share one concurrency
    /// limit. Authorization and network errors fail the call; any other API error drops the node
    /// it hit, or, for a shape, the test (listed in [`CourseTests::skipped`]).
    pub async fn course_tests(&self, term_ids: &[String]) -> Result<CourseTests> {
        if term_ids.is_empty() {
            return Ok(CourseTests::default());
        }
        let params = [
            ("fields".to_owned(), PARTICIPANT_FIELDS.to_owned()),
            ("active_terms_only".to_owned(), "false".to_owned()),
        ];
        let value = self
            .call_authenticated("crstests/participant2", &params)
            .await?;
        let wanted: Vec<&str> = term_ids.iter().map(String::as_str).collect();
        let mut tests = parse_participants(&value, &wanted)?;

        // Phase 1: the shapes of all tests.
        // The ids are collected first: a closure over `&TestRoot` held in the stream makes the
        // future fail rustc's higher-ranked `Send` check ("FnOnce is not general enough").
        let root_ids: Vec<String> = tests.iter().map(|t| t.root.id.clone()).collect();
        let shapes: Vec<(usize, Result<Value>)> = stream::iter(root_ids.into_iter().enumerate())
            .map(|(i, id)| async move { (i, self.crstests_node(&id, SHAPE_FIELDS).await) })
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await;
        let mut parsed: Vec<Option<RawShape>> = tests.iter().map(|_| None).collect();
        for (i, result) in shapes {
            match result {
                Ok(v) => parsed[i] = serde_json::from_value(v).ok(),
                Err(e) if is_fatal(&e) => return Err(e),
                Err(_) => {}
            }
        }

        // Phase 2: every node of every readable test.
        let mut jobs = Vec::new();
        for (i, shape) in parsed.iter().enumerate() {
            if let Some(shape) = shape {
                let mut ids = Vec::new();
                shape_ids(shape, &mut ids);
                jobs.extend(ids.into_iter().map(|id| (i, id)));
            }
        }
        let fetched: Vec<(usize, String, Result<Value>)> = stream::iter(jobs)
            .map(|(i, id)| async move {
                let value = self.crstests_node(&id, NODE_FIELDS).await;
                (i, id, value)
            })
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await;
        let mut details: Vec<Vec<(String, Value)>> = tests.iter().map(|_| Vec::new()).collect();
        for (i, id, result) in fetched {
            match result {
                Ok(v) => details[i].push((id, v)),
                Err(e) if is_fatal(&e) => return Err(e),
                Err(_) => {}
            }
        }

        let mut out = CourseTests::default();
        for ((mut test, shape), details) in tests.drain(..).zip(parsed).zip(details) {
            match shape {
                Some(shape) => {
                    assemble(&mut test.root, &shape, details);
                    out.tests.push(test);
                }
                None => out.skipped.push(test.root.id),
            }
        }
        Ok(out)
    }

    async fn crstests_node(&self, id: &str, fields: &str) -> Result<Value> {
        let params = [
            ("node_id".to_owned(), id.to_owned()),
            ("fields".to_owned(), fields.to_owned()),
        ];
        self.call_authenticated("crstests/node2", &params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assert_send<T: Send>(_: T) {}

    /// The futures must be `Send`, so a host can run them on a multi-threaded runtime. Compile-time
    /// only: nothing is awaited.
    #[test]
    fn client_futures_are_send() {
        let client = Client::new("https://example.invalid/", "k", "s").unwrap();
        let ids = vec!["1".to_owned()];
        let day = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        assert_send(client.course_tests(&ids));
        assert_send(client.term_grades(&ids));
        assert_send(client.timetable_range(day, day));
    }

    fn task(id: &str, points: Option<f64>, max: Option<f64>) -> TestNode {
        TestNode {
            id: id.to_owned(),
            kind: NodeKind::Task,
            points,
            points_max: max,
            ..TestNode::default()
        }
    }

    fn folder(id: &str, max: Option<f64>, children: Vec<TestNode>) -> TestNode {
        TestNode {
            id: id.to_owned(),
            kind: NodeKind::Folder,
            points_max: max,
            children,
            ..TestNode::default()
        }
    }

    #[test]
    fn models_roundtrip_through_json() {
        let grade_node = TestNode {
            id: "3".to_owned(),
            kind: NodeKind::Grade,
            hidden: true,
            last_changed: chrono::NaiveDate::from_ymd_opt(2026, 11, 4)
                .and_then(|d| d.and_hms_opt(9, 30, 0)),
            grade: Some(NodeGrade {
                symbol: Some("4,5".to_owned()),
                passes: Some(true),
                value: Some(4.5),
                ..NodeGrade::default()
            }),
            ..TestNode::default()
        };
        let tests = CourseTests {
            tests: vec![TestRoot {
                course_id: "C".to_owned(),
                course_name: LangDict {
                    pl: Some("Kurs".to_owned()),
                    en: None,
                },
                term_id: "2026/2027-Z".to_owned(),
                limited_to_groups: true,
                root: TestNode {
                    id: "1".to_owned(),
                    kind: NodeKind::Root,
                    children: vec![
                        folder("2", Some(10.0), vec![task("4", Some(5.0), Some(10.0))]),
                        grade_node,
                    ],
                    ..TestNode::default()
                },
            }],
            skipped: vec!["9".to_owned()],
        };
        let json = serde_json::to_string(&tests).unwrap();
        assert!(json.contains(r#""kind":"grade""#));
        assert_eq!(serde_json::from_str::<CourseTests>(&json).unwrap(), tests);
        // A node type added by a later version is read as `Unknown`.
        let node: TestNode = serde_json::from_str(r#"{"id":"1","kind":"quiz"}"#).unwrap();
        assert_eq!(node.kind, NodeKind::Unknown);
    }

    #[test]
    fn sums_skip_missing_values() {
        assert_eq!(sum_points(&[]), None);
        assert_eq!(sum_points(&[task("1", None, Some(5.0))]), None);
        let nodes = [
            task("1", Some(2.5), Some(5.0)),
            task("2", None, Some(5.0)),
            task("3", Some(0.0), None),
        ];
        assert_eq!(sum_points(&nodes), Some(2.5));
        assert_eq!(sum_max(&nodes), Some(10.0));
    }

    #[test]
    fn aggregates_folders_and_root() {
        let mut root = TestNode {
            kind: NodeKind::Root,
            children: vec![
                folder(
                    "10",
                    Some(20.0),
                    vec![
                        task("11", Some(4.0), Some(10.0)),
                        task("12", None, Some(10.0)),
                    ],
                ),
                folder("20", None, vec![task("21", Some(6.0), Some(8.0))]),
                folder("30", None, vec![]),
                task("40", Some(1.0), Some(2.0)),
            ],
            ..TestNode::default()
        };
        aggregate(&mut root);
        let f = &root.children;
        // USOS' own folder maximum wins over the sum of its tasks.
        assert_eq!((f[0].points, f[0].points_max), (Some(4.0), Some(20.0)));
        assert_eq!((f[1].points, f[1].points_max), (Some(6.0), Some(8.0)));
        assert_eq!((f[2].points, f[2].points_max), (None, None));
        assert_eq!((root.points, root.points_max), (Some(11.0), Some(30.0)));
    }

    #[test]
    fn aggregates_the_latest_change() {
        let at = |s: &str| s.parse::<NaiveDateTime>().ok();
        let mut a = task("1", Some(1.0), None);
        a.last_changed = at("2026-11-01T10:00:00");
        let mut b = task("2", Some(1.0), None);
        b.last_changed = at("2026-11-03T08:30:00");
        let mut root = folder("0", None, vec![a, b]);
        aggregate(&mut root);
        assert_eq!(root.last_changed, at("2026-11-03T08:30:00"));
    }

    #[test]
    fn kinds() {
        assert_eq!(NodeKind::parse("task"), NodeKind::Task);
        assert_eq!(NodeKind::parse(" Grade "), NodeKind::Grade);
        assert_eq!(NodeKind::parse("quiz"), NodeKind::Unknown);
        assert_eq!(serde_json::to_value(NodeKind::Folder).unwrap(), "folder");
    }

    /// The shape of `participant2` as documented (`course_test` fields), with a limited test, a
    /// foreign term, and broken items.
    fn participants() -> Value {
        json!([
            {
                "course_edition": {"course": {"id": "103B-INZ", "name": {"pl": "Analiza", "en": "Analysis"}},
                                   "term": {"id": "2026/2027-Z"}},
                "is_limited_to_groups": true,
                "root": {"id": 9001, "name": {"pl": "Zaliczenie", "en": "Credit"}, "description": {"pl": "", "en": ""}}
            },
            {
                "course_edition": {"course": {"id": "100A", "name": {"pl": "Algebra", "en": null}},
                                   "term": {"id": "2026/2027-Z"}},
                "root": {"id": "9000", "name": {"pl": "Algebra", "en": ""}}
            },
            {
                "course_edition": {"course": {"id": "1"}, "term": {"id": "2025/2026-L"}},
                "root": {"id": 1}
            },
            {"course_edition": {"course": {"id": "2"}, "term": {"id": "2026/2027-Z"}}},
            {"course_edition": null, "root": {"id": 3}},
            "garbage"
        ])
    }

    #[test]
    fn parses_participants() {
        let tests = parse_participants(&participants(), &["2026/2027-Z"]).unwrap();
        assert_eq!(tests.len(), 2);
        // Sorted by course id.
        assert_eq!(tests[0].course_id, "100A");
        assert_eq!(tests[0].root.id, "9000");
        assert!(!tests[0].limited_to_groups);
        assert_eq!(tests[1].course_id, "103B-INZ");
        assert_eq!(tests[1].root.id, "9001");
        assert!(tests[1].limited_to_groups);
        assert_eq!(tests[1].root.kind, NodeKind::Root);
        assert_eq!(tests[1].course_name.en.as_deref(), Some("Analysis"));
        assert!(
            parse_participants(&participants(), &["x"])
                .unwrap()
                .is_empty()
        );
        assert!(parse_participants(&json!([]), &["x"]).unwrap().is_empty());
        assert!(parse_participants(&json!({}), &["x"]).is_err());
    }

    /// `node2` fixtures in the shape of the documented fields (and of live public nodes).
    fn details() -> Vec<(String, Value)> {
        let v = |id: &str, value: Value| (id.to_owned(), value);
        vec![
            v(
                "12",
                json!({"id": 12, "type": "task", "order": 2, "name": {"pl": "Zadanie 2", "en": ""},
                    "results_visible_to_students": true, "folder_node_details": null, "grade_node_details": null,
                    "task_node_details": {"points_min": 0.0, "points_max": 10.0, "points_precision": 1,
                        "students_points": {"points": null, "comment": null, "last_changed": null}}}),
            ),
            v(
                "11",
                json!({"id": 11, "type": "task", "order": 1, "name": {"pl": "Zadanie 1", "en": "Task 1"},
                    "task_node_details": {"points_max": "10,0",
                        "students_points": {"points": 7.5, "comment": "ok", "last_changed": "2026-11-02 12:00:00"}}}),
            ),
            v(
                "10",
                json!({"id": 10, "type": "folder", "order": 1, "name": {"pl": "Kolokwium", "en": ""},
                    "folder_node_details": {"points_min": 0.0, "points_max": 20.0}, "task_node_details": null}),
            ),
            v(
                "13",
                json!({"id": 13, "type": "grade", "order": null, "name": {"pl": "Ocena", "en": "Grade"},
                    "grade_node_details": {"grade_type": {"id": "OCENA"},
                        "students_grade": {"grade_value": {"symbol": "4,5", "passes": true, "decimal_value": 4.5,
                            "name": {"pl": "cztery i pół", "en": "four and a half"}},
                            "comment": null, "last_changed": "2026-11-03 09:00:00"}}}),
            ),
            v("14", json!({"id": 14, "type": "mystery", "order": 3})),
        ]
    }

    fn shape() -> RawShape {
        serde_json::from_value(json!({
            "id": 9001, "type": "root",
            "subnodes_deep": [
                {"id": 13, "type": "grade", "subnodes_deep": []},
                {"id": 10, "type": "folder", "subnodes_deep": [
                    {"id": 12, "type": "task", "subnodes_deep": []},
                    {"id": 11, "type": "task", "subnodes_deep": []}
                ]},
                {"id": 14, "subnodes_deep": []},
                {"subnodes_deep": []},
                {"id": 99, "subnodes_deep": []}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn builds_the_tree() {
        let shape = shape();
        let mut ids = Vec::new();
        shape_ids(&shape, &mut ids);
        assert_eq!(ids, ["13", "10", "12", "11", "14", "99"]);

        let mut root = TestNode {
            id: "9001".into(),
            kind: NodeKind::Root,
            ..TestNode::default()
        };
        assemble(&mut root, &shape, details());
        // Order 1 (folder 10), 3 (unknown 14), then no order (grade 13); 99 has no details.
        let ids: Vec<&str> = root.children.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["10", "14", "13"]);
        let folder = &root.children[0];
        assert_eq!(folder.kind, NodeKind::Folder);
        let ids: Vec<&str> = folder.children.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["11", "12"]);
        assert_eq!(folder.children[0].points, Some(7.5));
        assert_eq!(folder.children[0].points_max, Some(10.0));
        assert_eq!(folder.children[0].comment.as_deref(), Some("ok"));
        assert_eq!(folder.children[1].points, None);
        assert_eq!(folder.children[1].points_max, Some(10.0));
        assert_eq!((folder.points, folder.points_max), (Some(7.5), Some(20.0)));
        assert_eq!(folder.last_changed, "2026-11-02T12:00:00".parse().ok());
        assert_eq!(root.children[1].kind, NodeKind::Unknown);
        let grade = &root.children[2];
        assert_eq!(grade.kind, NodeKind::Grade);
        assert_eq!(grade.points, None);
        let g = grade.grade.as_ref().unwrap();
        assert_eq!(g.symbol.as_deref(), Some("4,5"));
        assert_eq!(g.value, Some(4.5));
        assert_eq!(g.passes, Some(true));
        assert_eq!(grade.last_changed, "2026-11-03T09:00:00".parse().ok());
        assert_eq!((root.points, root.points_max), (Some(7.5), Some(20.0)));
        assert_eq!(root.last_changed, grade.last_changed);
    }

    #[test]
    fn empty_and_missing_values() {
        // A grade node without a grade yet, as the live API returns it.
        let mut root = TestNode::default();
        let shape: RawShape =
            serde_json::from_value(json!({"id": 1, "subnodes_deep": [{"id": 2}]})).unwrap();
        let details = vec![(
            "2".to_owned(),
            json!({"id": 2, "type": "grade", "order": 1, "name": {"pl": "Ocena", "en": ""},
                "grade_node_details": {"grade_type": {"id": "ZAL"},
                    "students_grade": {"comment": null, "grade_value": null, "last_changed": null}}}),
        )];
        assemble(&mut root, &shape, details);
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].grade, None);
        assert_eq!(root.points, None);
        // No children at all.
        let mut empty = TestNode::default();
        let shape: RawShape =
            serde_json::from_value(json!({"id": 1, "subnodes_deep": []})).unwrap();
        assemble(&mut empty, &shape, vec![]);
        assert!(empty.children.is_empty());
    }

    #[test]
    fn hidden_nodes_leave_the_totals() {
        let mut secret = task("12", Some(9.0), Some(10.0));
        secret.hidden = true;
        let inner = folder(
            "20",
            Some(30.0),
            vec![task("21", Some(1.0), Some(4.0)), secret.clone()],
        );
        let clean = folder("30", Some(6.0), vec![task("31", Some(2.0), Some(6.0))]);
        let mut root = TestNode {
            kind: NodeKind::Root,
            children: vec![
                folder(
                    "10",
                    Some(50.0),
                    vec![task("11", Some(3.0), Some(5.0)), secret, inner],
                ),
                clean,
            ],
            ..TestNode::default()
        };
        aggregate(&mut root);
        let (f10, f30) = (&root.children[0], &root.children[1]);
        // Hidden below (also two levels down): the visible maxima, not USOS's.
        assert_eq!(
            (f10.children[2].points, f10.children[2].points_max),
            (Some(1.0), Some(4.0))
        );
        assert_eq!((f10.points, f10.points_max), (Some(4.0), Some(9.0)));
        // Nothing hidden below: USOS's maximum stays.
        assert_eq!((f30.points, f30.points_max), (Some(2.0), Some(6.0)));
        assert_eq!((root.points, root.points_max), (Some(6.0), Some(15.0)));
        // The hidden node itself is kept.
        assert!(f10.children[1].hidden);
        assert_eq!(f10.children[1].points, Some(9.0));
        assert_eq!(sum_points(&f10.children), Some(4.0));

        let raw: RawNode = serde_json::from_value(
            json!({"id": 1, "type": "task", "results_visible_to_students": false}),
        )
        .unwrap();
        assert!(raw.into_node("1").1.hidden);
    }

    #[test]
    fn fatal_errors() {
        let api = |status| {
            Error::Api(crate::error::ApiError::from_response(
                status,
                "{}",
                str::to_owned,
            ))
        };
        assert!(is_fatal(&api(401)));
        assert!(is_fatal(&Error::NotLoggedIn));
        assert!(!is_fatal(&api(400)));
        assert!(!is_fatal(&api(500)));
    }

    #[test]
    fn shape_ids_are_deduplicated() {
        let shape: RawShape = serde_json::from_value(json!({"id": 1, "subnodes_deep": [
            {"id": 2, "subnodes_deep": [{"id": 3}, {"id": 2}]}, {"id": 3}, {"id": 4}]}))
        .unwrap();
        let mut ids = Vec::new();
        shape_ids(&shape, &mut ids);
        assert_eq!(ids, ["2", "3", "4"]);
    }

    #[test]
    fn serializes_with_clean_names() {
        let v = serde_json::to_value(task("5", Some(1.0), Some(2.0))).unwrap();
        assert_eq!(v["kind"], "task");
        assert_eq!(v["points"], 1.0);
        assert_eq!(v["points_max"], 2.0);
        assert!(v["children"].as_array().unwrap().is_empty());
    }
}
