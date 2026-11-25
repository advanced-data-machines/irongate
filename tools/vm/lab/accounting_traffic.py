"""Guest-only known IP-octet workload. No portal or backend socket shares its port."""
import json
import socket
import sys
import time
from guest_common import WORK,guard


def main(mode,address,count,length,interval):
    guard((WORK/"id").read_text().strip())
    with socket.socket(socket.AF_INET,socket.SOCK_DGRAM)as sock:
        sock.settimeout(5)
        if mode=="receive":
            sock.bind((address,9900));print("ready",flush=True)
            received=0
            for _ in range(count):
                payload,_=sock.recvfrom(2048)
                assert len(payload)==length
                received+=len(payload)+28
            print(json.dumps({"ip_octets":received,"packets":count}),flush=True)
        else:
            payload=b'i'*length
            for _ in range(count):
                assert sock.sendto(payload,(address,9900))==length
                if interval:time.sleep(interval)
            print(json.dumps({"ip_octets":count*(length+28),"packets":count}),flush=True)


if __name__=="__main__":main(sys.argv[1],sys.argv[2],int(sys.argv[3]),int(sys.argv[4]),float(sys.argv[5]))
