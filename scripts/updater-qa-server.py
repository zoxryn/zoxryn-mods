"""Loopback-only test server. Never used by production updater builds."""
from http.server import ThreadingHTTPServer,BaseHTTPRequestHandler
from pathlib import Path
import argparse,json,threading,time
p=argparse.ArgumentParser();p.add_argument('root');a=p.parse_args();root=Path(a.root).resolve();lock=threading.Lock();requests=[]
class Handler(BaseHTTPRequestHandler):
 def log_message(self,*args):pass
 def do_GET(self):
  scenario=(root/'scenario.txt').read_text().strip() if (root/'scenario.txt').exists() else 'valid'
  with lock:
   requests.append({'scenario':scenario,'path':self.path});(root/'requests.json').write_text(json.dumps(requests,indent=2))
  if scenario=='network_error':self.send_error(503);return
  if scenario=='missing_release':self.send_error(404);return
  manifest_case={'expired_metadata':'expired','downgrade':'downgrade','prerelease':'prerelease','invalid_manifest':'invalid','wrong_artifact_version':'wrong-artifact-version'}.get(scenario,'latest')
  if self.path=='/latest.json':
   data=(root/(manifest_case+'.json')).read_bytes()
   if scenario=='bad_manifest':data=data.replace(b'QA signed application update',b'QA forged application update')
  elif self.path=='/latest.json.sig':data=(root/(manifest_case+'.json.sig')).read_bytes()
  elif self.path=='/installer.exe':
   data=(root/'candidate.exe').read_bytes()
   if scenario=='tampered_installer':data=data[:-1]+bytes([data[-1]^1])
  else:self.send_error(404);return
  self.send_response(200);self.send_header('Content-Length',str(len(data)));self.send_header('Content-Type','application/octet-stream');self.end_headers()
  if scenario=='interrupted_download' and self.path=='/installer.exe':
   self.wfile.write(data[:len(data)//2]);self.wfile.flush();self.close_connection=True;return
  for offset in range(0,len(data),65536):
   self.wfile.write(data[offset:offset+65536]);self.wfile.flush()
   if self.path=='/installer.exe':time.sleep(0.015)
server=ThreadingHTTPServer(('127.0.0.1',0),Handler);(root/'port.txt').write_text(str(server.server_port));server.serve_forever()
