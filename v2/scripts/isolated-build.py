#!/usr/bin/env python3
import pathlib,subprocess,shutil,os,argparse
root=pathlib.Path(__file__).resolve().parents[1];p=argparse.ArgumentParser();p.add_argument('--name',default='isolation');a=p.parse_args();assert a.name.replace('-','').isalnum();qa=root/'qa'/a.name;qa.mkdir(parents=True,exist_ok=True)
source=qa/'source';assert not source.exists(),'Use a new isolated directory for a genuinely fresh run; do not reuse it.'
shutil.copytree(root,source,ignore=shutil.ignore_patterns('qa','target','dist'))
bin_dir=pathlib.Path(subprocess.check_output(['rustup','which','--toolchain','stable','cargo'],text=True).strip()).parent
env=dict(os.environ,CARGO_PROFILE_RELEASE_STRIP='none',HOME=str(qa/'home'),CARGO_HOME=str(qa/'cargo-home'),CARGO_TARGET_DIR=str(qa/'target'),TMPDIR=str(qa/'tmp'),PATH=str(bin_dir)+':/usr/bin:/bin:/opt/homebrew/bin')
for key in ['HOME','CARGO_HOME','TMPDIR']:pathlib.Path(env[key]).mkdir(parents=True,exist_ok=True)
assert not any((pathlib.Path(env['CARGO_HOME'])).iterdir())
subprocess.run([str(bin_dir/'cargo'),'build','--locked','--release'],cwd=source,env=env,check=True)
print('Fresh Cargo home isolated HTTPS SDK source build passed:',qa/'target/release/fontawesome-v2')
