"""Drive guest Chromium over DevTools pipes; no debugging port or host browser."""

import base64
import json
import os
import select
import signal
import subprocess
import time
from guest_common import CLIENT,WORK,guard,spawn


class Browser:
    def __init__(self,children):
        guard((WORK/"id").read_text().strip())
        incoming,outgoing=WORK/"cdp-input",WORK/"cdp-output"
        for path in (incoming,outgoing):os.mkfifo(path,0o600)
        self.log=(WORK/"browser.log").open("w")
        self.proc=spawn("bash","-c",'exec 3<"$1" 4>"$2"; shift 2; exec chromium "$@"',"ig-browser",str(incoming),str(outgoing),
            "--headless","--no-sandbox","--disable-gpu","--disable-dev-shm-usage","--no-proxy-server",
            "--disable-background-networking","--no-first-run","--disable-default-apps","--disable-sync",
            "--remote-debugging-pipe",f"--user-data-dir={WORK/'chromium'}","--window-size=900,820","about:blank",
            namespace=CLIENT,stdin=subprocess.DEVNULL,stdout=self.log,stderr=self.log,start_new_session=True)
        children.append(self.proc)
        deadline=time.monotonic()+5
        while True:
            try:
                self.tx=os.open(incoming,os.O_WRONLY|os.O_NONBLOCK);break
            except OSError:
                if time.monotonic()>deadline:raise RuntimeError("Chromium pipe did not open")
                time.sleep(.02)
        self.rx=os.open(outgoing,os.O_RDONLY|os.O_NONBLOCK)
        self.buffer=b"";self.sequence=0;self.session=None
        self.version=self.call("Browser.getVersion")["product"]
        target=self.call("Target.createTarget",{"url":"about:blank"})["targetId"]
        self.session=self.call("Target.attachToTarget",{"targetId":target,"flatten":True})["sessionId"]
        self.call("Page.enable")
        self.call("Runtime.enable")

    def call(self,method,params=None):
        self.sequence+=1
        request={"id":self.sequence,"method":method,"params":params or {}}
        if self.session and not method.startswith(("Browser.","Target.")):request["sessionId"]=self.session
        encoded=json.dumps(request).encode()+b"\0"
        os.write(self.tx,encoded)
        deadline=time.monotonic()+8
        while time.monotonic()<deadline:
            while b"\0" in self.buffer:
                frame,self.buffer=self.buffer.split(b"\0",1)
                message=json.loads(frame)
                if message.get("id")==self.sequence:
                    if "error"in message:raise RuntimeError(f"Browser command failed: {method}")
                    return message.get("result",{})
            if select.select([self.rx],[],[],.1)[0]:
                data=os.read(self.rx,65536)
                if not data:
                    if self.proc.poll() is not None:raise RuntimeError("Chromium exited")
                    time.sleep(.01)
                self.buffer+=data
                if len(self.buffer)>2*1024*1024:raise RuntimeError("Browser message exceeds bound")
        raise RuntimeError(f"Browser command timed out: {method}")

    def evaluate(self,expression):
        result=self.call("Runtime.evaluate",{"expression":expression,"returnByValue":True})
        if "exceptionDetails"in result:raise RuntimeError("Browser expression failed")
        return result.get("result",{}).get("value")

    def navigate(self):
        self.evaluate("window.__igNavigationPending=true;true")
        self.call("Page.navigate",{"url":"http://10.50.10.1:8080/"})
        self.wait("!window.__igNavigationPending && document.readyState==='complete' && !!document.querySelector('form')")

    def wait(self,expression,seconds=5):
        deadline=time.monotonic()+seconds
        while time.monotonic()<deadline:
            if self.evaluate(expression):return
            time.sleep(.05)
        raise RuntimeError("Browser did not reach expected portal state")

    def submit(self):
        self.evaluate("document.querySelector('form').requestSubmit();true")

    def screenshot(self,name):
        encoded=self.call("Page.captureScreenshot",{"format":"png"})["data"]
        data=base64.b64decode(encoded,validate=True)
        if len(data)>1024*1024:raise RuntimeError("Screenshot exceeds bound")
        (WORK/name).write_bytes(data)

    def close(self):
        try:self.call("Browser.close")
        except Exception:pass
        for fd in (getattr(self,"tx",None),getattr(self,"rx",None)):
            if fd is not None:os.close(fd)
        try:self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(self.proc.pid,signal.SIGKILL);self.proc.wait()
        self.log.close()
