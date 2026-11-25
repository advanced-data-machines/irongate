"""Read independent backend detail records; never export usernames or secrets."""
import re
from guest_common import WORK,GATEWAY,command


def rows():
    path=WORK/"radius/accounting-detail"
    if not path.exists():return []
    result=[]
    for block in path.read_text().split("\n\n"):
        attributes=dict(re.findall(r'^\s+([A-Za-z-]+) = (.+)$',block,re.M))
        if "Acct-Status-Type"not in attributes:continue
        def number(key):return int(attributes.get(key,"0"))
        result.append({"session_id":attributes["Acct-Session-Id"].strip('"'),"kind":attributes["Acct-Status-Type"],
            "input":number("Acct-Input-Octets")+(number("Acct-Input-Gigawords")<<32),
            "output":number("Acct-Output-Octets")+(number("Acct-Output-Gigawords")<<32),
            "seconds":number("Acct-Session-Time"),"delay":number("Acct-Delay-Time"),"cause":attributes.get("Acct-Terminate-Cause"),
            "identity_matches":attributes.get("User-Name","").strip('"')=="lab-user"})
    return result


def drop_responses(enabled):
    if enabled:
        command("nft","-f","-",namespace=GATEWAY,input='table inet ig_accounting_fault { chain input { type filter hook input priority -10; policy accept; ip saddr 192.0.2.80 udp sport 1813 drop; }\n}\n')
    else:command("nft","delete","table","inet","ig_accounting_fault",namespace=GATEWAY)
