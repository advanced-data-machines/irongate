"""One explicit native portal mode shared by owned guest helpers."""
import ssl
from guest_common import WORK,guard


def secure():
    return (WORK/"mode").read_text().strip()in ("https","radius","accounting")


def port():
    return 8443 if secure() else 8080


def origin():
    return f"{'https'if secure() else'http'}://10.50.10.1:{port()}"


def context():
    return ssl.create_default_context(cafile=str(WORK/"tls/ca.pem"))


def configure(run_id):
    guard(run_id)
    text=f'mode = "{"https"if secure() else"http-lab"}"\norigin = "{origin()}"\n'
    if secure():
        from certificates import prepare
        prepare(run_id)
        text+=f'certificate_der = "{WORK}/tls/server.der"\nprivate_key_der = "{WORK}/tls/server.key.der"\n'
    if (WORK/"mode").read_text().strip()in ("radius","accounting"):
        from radius_backend import configure as radius_configure
        radius_configure(run_id)
        text+=f'provider = "radius"\nradius_endpoint = "192.0.2.80:1812"\nradius_secret_file = "{WORK}/radius/secret"\nnas_identifier = "irongate-lab"\n'
    if (WORK/"mode").read_text().strip()=="accounting":
        text+='accounting = "radius"\naccounting_endpoint = "192.0.2.80:1813"\naccounting_interim_seconds = 2\n'
    (WORK/"portal.toml").write_text(text)
    (WORK/"portal.toml").chmod(0o600)
