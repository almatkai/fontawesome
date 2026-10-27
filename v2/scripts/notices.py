#!/usr/bin/env python3
"""Collect license evidence from immutable source and Cargo's locked package sources."""
import pathlib,subprocess,json,shutil,hashlib
root=pathlib.Path(__file__).resolve().parents[1]
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=root))
records=[]
for p in metadata['packages']:
 if p['name']=='fontawesome-v2':continue
 directory=pathlib.Path(p['manifest_path']).parent;out=root/'licenses'/f'{p["name"]}-{p["version"]}'
 files=[]
 for f in sorted(directory.iterdir()):
  if f.is_file() and (f.name.upper().startswith(('LICENSE','COPYING','NOTICE'))):
   out.mkdir(parents=True,exist_ok=True);shutil.copyfile(f,out/f.name);files.append({'path':str((out/f.name).relative_to(root)),'sha256':hashlib.sha256(f.read_bytes()).hexdigest()})
 records.append({'name':p['name'],'version':p['version'],'source':p['source'],'declaredLicense':p['license'],'evidence':files,'missingEvidence':not bool(files)})
for name,source in [('fontawesome-source',root/'fixtures/source/LICENSE'),('raycast-upstream',root/'fixtures/upstream/LICENSE')]:
 out=root/'licenses'/name;out.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,out/'LICENSE')
(root/'DEPENDENCIES.json').write_text(json.dumps({'includes':'Locked runtime, build and QA dependencies; declarations are evidence, not newly granted licenses.','packages':sorted(records,key=lambda p:(p['name'],p['version']))},indent=2)+'\n')
print('License declarations and original text evidence collected:',len(records),'packages;',sum(r['missingEvidence'] for r in records),'without top-level license text (documented).')
