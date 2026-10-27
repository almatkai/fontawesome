#!/usr/bin/env python3
import pathlib,json,hashlib
r=pathlib.Path(__file__).resolve().parents[1];lock=json.loads((r/'fixtures/source/package-lock.json').read_text());records=[]
for name,p in lock['packages'].items():
 if not name:continue
 files=[];directory=r/'qa/v1'/name
 if directory.is_dir():
  for f in sorted(directory.iterdir()):
   if f.is_file() and f.name.upper().startswith(('LICENSE','COPYING','NOTICE')):
    target=r/'fixtures/npm-licenses'/name.removeprefix('node_modules/').replace('/node_modules/','__').replace('/','__')/f.name;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(f.read_bytes());files.append({'path':str(target.relative_to(r)),'sha256':hashlib.sha256(f.read_bytes()).hexdigest()})
 records.append({'package':name,'version':p.get('version'),'resolved':p.get('resolved'),'integrity':p.get('integrity'),'licenseDeclaration':p.get('license'),'evidence':files,'missingEvidence':not files})
(r/'SOURCE-DEPENDENCIES.json').write_text(json.dumps({'scope':'Exact original package-lock.json; QA-only JavaScript dependencies are not shipped in the executable ZIP. Optional platform packages not installed have no local text evidence.','packages':records},indent=2)+'\n')
