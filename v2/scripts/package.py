#!/usr/bin/env python3
"""Deterministic private development ZIP, no release URLs or publication."""
import argparse,pathlib,hashlib,json,zipfile,stat,os,shutil,subprocess
root=pathlib.Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--binary',type=pathlib.Path,default=root/'target/release/fontawesome-v2');p.add_argument('--target',default='aarch64-apple-darwin',choices=['aarch64-apple-darwin','x86_64-pc-windows-msvc']);args=p.parse_args()
exe='bin/fontawesome-v2'+('.exe' if 'windows' in args.target else '')
files={exe:args.binary.read_bytes(),'LICENSE':(root/'LICENSE').read_bytes(),'NOTICE':(root/'NOTICE').read_bytes()}
for name in ['PROVENANCE.json','DEPENDENCIES.json']:
 files['resources/provenance/'+name]=(root/name).read_bytes()
for base in ['resources','licenses']:
 for f in sorted((root/base).rglob('*')):
  if f.is_file():assert not f.is_symlink();files[str(f.relative_to(root))]=f.read_bytes()
sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=json.loads((root/'extension.json').read_text());assert manifest['artifacts']=={}
manifest['files']={name:sha(data) for name,data in sorted(files.items())};files['extension.json']=(json.dumps(manifest,sort_keys=True,indent=2)+'\n').encode()
out=root/'dist';out.mkdir(exist_ok=True);archive=out/(args.target+'.zip')
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_STORED) as z:
 for name,data in sorted(files.items()):
  info=zipfile.ZipInfo(name,(2020,1,1,0,0,0));info.create_system=3;info.external_attr=((stat.S_IFREG|(0o755 if name==exe else 0o644))<<16);z.writestr(info,data)
extract=out/('extracted-'+args.target);extract.mkdir(exist_ok=True)
with zipfile.ZipFile(archive) as z:
 assert z.namelist()==sorted(files)
 for name,data in files.items():
  assert z.read(name)==data
  file=extract/name;file.parent.mkdir(parents=True,exist_ok=True);file.write_bytes(data);file.chmod(0o755 if name==exe else 0o644)
inventory={'kind':'private-development-package','target':args.target,'archive':archive.name,'sha256':sha(archive.read_bytes()),'executable':exe,'executableSha256':sha(files[exe]),'files':{name:{'sha256':sha(data),'bytes':len(data),'mode':'0755' if name==exe else '0644'} for name,data in sorted(files.items())},'artifacts':{},'published':False}
(root/'PACKAGE-INVENTORY.json').write_text(json.dumps(inventory,indent=2,sort_keys=True)+'\n')
print('Private deterministic ZIP:',archive,'SHA256',inventory['sha256'])
