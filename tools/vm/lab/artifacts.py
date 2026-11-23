"""Bounded screenshot export over the serial log, without host directory shares."""
import base64
import hashlib
import json

NAMES=("portal-before.png","portal-connected.png","portal-logged-out.png")


def export(directory,run_id):
    metadata={}
    for name in NAMES:
        path=directory/name
        if not path.exists():continue
        data=path.read_bytes()
        if len(data)>1024*1024:raise RuntimeError("Screenshot exceeds bound")
        encoded=base64.b64encode(data).decode()
        chunks=[encoded[i:i+1024]for i in range(0,len(encoded),1024)]
        for index,chunk in enumerate(chunks):
            print("IG_LAB_ARTIFACT "+json.dumps({"run_id":run_id,"name":name,"part":index,"total":len(chunks),"data":chunk}),flush=True)
        metadata[name]=hashlib.sha256(data).hexdigest()
    return metadata


def extract(directory,run_id,expected):
    if set(expected)!=set(NAMES):raise ValueError("Missing required browser screenshots")
    pieces={name:{}for name in NAMES};totals={}
    path=directory/("evidence.log"if (directory/"evidence.log").exists() else "serial.log")
    for line in path.read_text(errors="replace").splitlines():
        if "IG_LAB_ARTIFACT "not in line:continue
        item=json.loads(line.split("IG_LAB_ARTIFACT ",1)[1]);name=item.get("name")
        if item.get("run_id")!=run_id or name not in pieces:raise ValueError("Unexpected screenshot identity")
        total,part=item["total"],item["part"]
        if type(total) is not int or type(part) is not int or not isinstance(item.get("data"),str) or len(item["data"])>1024:
            raise ValueError("Invalid screenshot chunk types/size")
        if not 1<=total<=2048 or not 0<=part<total or part in pieces[name] or totals.get(name,total)!=total:
            raise ValueError("Invalid/duplicate screenshot chunk")
        totals[name]=total;pieces[name][part]=item["data"]
    for name in NAMES:
        if len(pieces[name])!=totals.get(name):raise ValueError("Incomplete screenshot")
        data=base64.b64decode("".join(pieces[name][i]for i in range(totals[name])),validate=True)
        if len(data)>1024*1024 or not data.startswith(b"\x89PNG\r\n\x1a\n") or hashlib.sha256(data).hexdigest()!=expected[name]:
            raise ValueError("Screenshot format/digest mismatch")
        (directory/name).write_bytes(data)
