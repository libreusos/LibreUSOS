"""Watch the Ghostex CEF browser over CDP and capture an OAuth redirect to a custom scheme.

Intercepts usosapi responses at the Response stage, records the first Location header that
starts with the given scheme, writes it to the output file (mode 0600), and exits.
Never prints the captured URL.
"""
import asyncio
import json
import os
import sys
import urllib.request

import websockets

PORT = 9334
SCHEME = "libreusos:"
OUT = sys.argv[1]


async def main():
    info = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/version"))
    async with websockets.connect(info["webSocketDebuggerUrl"], max_size=None) as ws:
        next_id = 0

        async def send(method, params=None, session=None):
            nonlocal next_id
            next_id += 1
            msg = {"id": next_id, "method": method, "params": params or {}}
            if session:
                msg["sessionId"] = session
            await ws.send(json.dumps(msg))

        await send("Target.setDiscoverTargets", {"discover": True})
        await send("Target.setAutoAttach", {"autoAttach": True, "waitForDebuggerOnStart": False, "flatten": True})
        attached = set()
        print("watching", flush=True)
        async for raw in ws:
            msg = json.loads(raw)
            method = msg.get("method")
            params = msg.get("params", {})
            if method == "Target.targetCreated" or method == "Target.targetInfoChanged":
                t = params["targetInfo"]
                if t["type"] == "page" and t["targetId"] not in attached:
                    attached.add(t["targetId"])
                    await send("Target.attachToTarget", {"targetId": t["targetId"], "flatten": True})
            elif method == "Target.attachedToTarget":
                sid = params["sessionId"]
                await send("Fetch.enable", {"patterns": [
                    {"urlPattern": "*usosapi.polsl.pl/*", "requestStage": "Response"}]}, sid)
            elif method == "Fetch.requestPaused":
                sid = msg.get("sessionId")
                loc = ""
                for h in params.get("responseHeaders") or []:
                    if h["name"].lower() == "location":
                        loc = h["value"]
                await send("Fetch.continueRequest", {"requestId": params["requestId"]}, sid)
                if loc.startswith(SCHEME):
                    fd = os.open(OUT, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
                    with os.fdopen(fd, "w") as f:
                        f.write(loc)
                    print("captured", flush=True)
                    return


asyncio.run(main())
