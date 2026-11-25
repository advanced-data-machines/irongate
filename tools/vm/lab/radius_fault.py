"""Clearly labeled protocol fault fixture; never presented as real backend auth."""
import hashlib
import hmac
import json
import socket
import time
from guest_common import WORK,namespace_guard

namespace_guard()
secret=(WORK/"radius/secret").read_bytes().strip()

def response(request,mode):
    code=11 if mode=="challenge"else 2
    attributes=b""if code==11 else bytes([27,6])+int(5).to_bytes(4,"big")
    if mode!="missing_ma":attributes+=bytes([80,18])+bytes(16)
    packet=bytearray(bytes([code,request[1]])+(20+len(attributes)).to_bytes(2,"big")+request[4:20]+attributes)
    if mode=="wrong_id":packet[1]^=1
    if mode=="length":packet[2:4]=bytes([0,1])
    if mode!="missing_ma":
        packet[-16:]=hmac.digest(secret,packet,"md5")
        if mode=="bad_ma":packet[-1]^=1
    packet[4:20]=hashlib.md5(packet+secret).digest()
    if mode=="bad_auth":packet[4]^=1
    return packet

with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as sock:
    sock.bind(("192.0.2.80",1812));count=0
    while True:
        request,peer=sock.recvfrom(4097)
        mode=json.loads((WORK/"radius/fault.json").read_text())["mode"]
        count+=1
        (WORK/"radius/fault-observed.json").write_text(json.dumps({"mode":mode,"received":count}))
        if mode=="timeout":continue
        if mode=="late":time.sleep(1.2)
        packet=response(request,mode)
        if mode=="wrong_source":
            with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as other:other.sendto(packet,peer)
        else:sock.sendto(packet,peer)
