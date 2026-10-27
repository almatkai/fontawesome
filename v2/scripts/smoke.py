#!/usr/bin/env python3
"""Real production executable pipe smoke. Invalid preference blocks all network/effects."""
import pathlib,subprocess,json,uuid,os,sys,tempfile,hashlib,zipfile
root=pathlib.Path(__file__).resolve().parents[1];inventory=json.loads((root/'PACKAGE-INVENTORY.json').read_text());directory=root/'dist'/('extracted-'+inventory['target']);binary=directory/inventory['executable']
for name,meta in inventory['files'].items():assert hashlib.sha256((directory/name).read_bytes()).hexdigest()==meta['sha256']
env={'PATH':'/usr/bin:/bin','HOME':str(root/'qa/home'),'TMPDIR':str(root/'qa/tmp')};(root/'qa/tmp').mkdir(parents=True,exist_ok=True)
for arg in ['--protocol-info','--self-test']:
 result=subprocess.run([str(binary),arg],env=env,capture_output=True,timeout=5,check=True);value=json.loads(result.stdout);assert value
with tempfile.TemporaryDirectory(dir=root/'qa/tmp') as temp:
 init='h:'+str(uuid.uuid4());opened='h:'+str(uuid.uuid4());shutdown='h:'+str(uuid.uuid4())
 frames=[{'type':'request','id':init,'method':'initialize','params':{'protocol':1,'extensionId':'com.tezbar.native.fontawesome','platform':'macOS','packageDir':str(directory),'dataDir':temp+'/data','tempDir':temp+'/tmp','grantedCapabilities':[]}}, {'type':'request','id':opened,'method':'session.open','params':{'sessionId':'production-smoke','commandName':'index','preferences':{'API_TOKEN':'\n'},'argumentValues':{}}},{'type':'request','id':shutdown,'method':'shutdown','params':{}}]
 # Sequential pipe, no host request may be issued before initialize ACK.
 child=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,text=True)
 try:
  hello=json.loads(child.stdout.readline());assert hello['type']=='hello' and hello['protocol']==1 and hello['id']=='com.tezbar.native.fontawesome' and hello['sdkVersion']=='0.1.1'
  for frame in frames:
   child.stdin.write(json.dumps(frame)+'\n');child.stdin.flush();reply=json.loads(child.stdout.readline());assert reply['type']=='response' and reply['id']==frame['id'];assert ('error' in reply) if frame['id']==opened else ('result' in reply)
  child.stdin.close();assert child.wait(timeout=5)==0
 finally:
  if child.poll() is None:child.kill();child.wait()
 # Protocol negatives must terminate, without issuing any reverse requests.
 for data in ['{}\n','{"type":"request","id":"bad-id","method":"initialize","params":{}}\n','{"bad":true}\r\n','{']:
  completed=subprocess.run([str(binary)],input=data,env=env,text=True,capture_output=True,timeout=5);assert completed.returncode!=0;lines=completed.stdout.splitlines();assert json.loads(lines[0])['type']=='hello';assert len(lines)==1
print('Extracted production executable: hashes, flags, initialize, credential validation before network, shutdown/EOF and malformed protocol rejection passed. No live API or clipboard.')
