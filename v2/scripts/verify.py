#!/usr/bin/env python3
"""Repeat verification within v2. Existing QA artifacts are retained for review."""
import pathlib,subprocess,os,json,shutil
root=pathlib.Path(__file__).resolve().parents[1];qa=root/'qa';qa.mkdir(exist_ok=True)
env=dict(os.environ,HOME=str(qa/'home'),TMPDIR=str(qa/'tmp'),npm_config_cache=str(qa/'npm-cache'),PNPM_HOME=str(qa/'pnpm-home'))
# Keep installed Rust tools available when isolating HOME.
env['RUSTUP_HOME']=os.environ.get('RUSTUP_HOME',str(pathlib.Path.home()/'.rustup'))
for key in ['HOME','TMPDIR','npm_config_cache','PNPM_HOME']:pathlib.Path(env[key]).mkdir(parents=True,exist_ok=True)
def run(args,**kw):subprocess.run(args,cwd=root,env=env,check=True,**kw)
source=qa/'v1'
if not source.exists():
 source.mkdir();archive=subprocess.Popen(['git','archive','39549ced40cbe73492213e731064404dde84d165'],cwd=root,stdout=subprocess.PIPE);subprocess.run(['tar','-x','-C',str(source)],stdin=archive.stdout,check=True);assert archive.wait()==0
run(['npm','ci','--legacy-peer-deps','--ignore-scripts','--no-audit','--no-fund','--prefix',str(source)])
base=['pnpm','--dir',str(source),'--config.verify-deps-before-run=false','--config.manage-package-manager-versions=false']
run(base+['run','test']);run(base+['run','typecheck']);run(base+['exec','tsx','--test','tests/*.test.ts'])
run(['node','scripts/oracle.cjs']);run(['python3','scripts/source-evidence.py'])
for toolchain in ['stable','1.85.0']:
 for args in [['fmt','--check'],['test','--locked'],['clippy','--locked','--all-targets','--','-D','warnings']]:run(['cargo','+'+toolchain]+args)
run(['node','scripts/contracts.cjs']);
venv=qa/'venv';
if not venv.exists():run(['python3','-m','venv',str(venv)])
python=venv/('Scripts/python.exe' if os.name=='nt' else 'bin/python')
run([str(python),'-m','pip','install','-r','qa-requirements.txt']);run([str(python),'scripts/schema.py']);run(['python3','scripts/host-patch.py'])
# The harness contains exact patched host functions and the tests shipped in the patch.
messages=subprocess.check_output(['cargo','+stable','test','--locked','--no-run','--message-format=json'],cwd=root,env=env,text=True)
artifacts=[json.loads(line) for line in messages.splitlines() if line.startswith('{')]
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1','--no-deps'],cwd=root,env=env));deps=pathlib.Path(metadata['target_directory'])/'debug/deps'
args=['rustup','run','stable','rustc','--edition=2021','--test','qa/host-registry-contract.rs','-L','dependency='+str(deps)]
for crate in ['serde_json','tempfile']:
 candidates=[f for m in artifacts if m.get('reason')=='compiler-artifact' and m['target']['name']==crate for f in m['filenames'] if f.endswith('.rlib')]
 assert candidates,crate
 args+=['--extern',crate+'='+candidates[-1]]
run(args+['-o','qa/host-registry-contract']);run([str(qa/'host-registry-contract')])
run(['cargo','build','--locked','--release']);run(['python3','scripts/notices.py']);run(['python3','scripts/package.py']);run(['python3','scripts/smoke.py']);run(['python3','scripts/inventory.py']);run(['python3','scripts/inventory.py','--check'])
print('All requested local checks passed. Optional fresh-home build: python3 scripts/isolated-build.py (requires a fresh qa/isolation directory).')
