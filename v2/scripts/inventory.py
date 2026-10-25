#!/usr/bin/env python3
import pathlib,subprocess,hashlib,argparse,json
root=pathlib.Path(__file__).resolve().parents[2];v2=root/'v2';p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args()
files=subprocess.check_output(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=root).decode().split('\0');files=sorted(set(x for x in files if x and x!='v2/INDEX.sha256'))
text=''.join(hashlib.sha256((root/f).read_bytes()).hexdigest()+'  '+f+'\n' for f in files)
if a.check:assert (v2/'INDEX.sha256').read_text()==text,'Git-visible source inventory changed'
else:(v2/'INDEX.sha256').write_text(text)
head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip();assert head=='39549ced40cbe73492213e731064404dde84d165'
assert not subprocess.check_output(['git','diff','--cached','--name-only'],cwd=root)
for name in subprocess.check_output(['git','ls-tree','-r','--name-only','HEAD'],cwd=root,text=True).splitlines():
 expected=subprocess.check_output(['git','show','HEAD:'+name],cwd=root)
 actual=(root/name).read_bytes()
 if name=='README.md':assert actual==expected+b'\nStandalone external Rust implementation: [V2](./v2/README.md).\n'
 else:assert actual==expected,'V1 changed: '+name
# Ignore rules must not hide any required source or fixture.
for f in v2.rglob('*'):
 if f.is_file() and not any(x in f.relative_to(v2).parts for x in ['qa','target','dist','__pycache__']):
  assert str(f.relative_to(root)) in files or f.name=='INDEX.sha256','Required file hidden: '+str(f)
print(f'{len(files)} Git-visible files inventoried; all V1 bytes except discovery link exact; HEAD and index unchanged.')
