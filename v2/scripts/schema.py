#!/usr/bin/env python3
"""Validate actual pinned schema and explicit negative cases, without a host build."""
import pathlib,json,copy
from jsonschema import Draft202012Validator,FormatChecker
root=pathlib.Path(__file__).resolve().parents[1];schema=json.loads((root/'fixtures/host/schemas/extension-v3.schema.json').read_text());v=Draft202012Validator(schema,format_checker=FormatChecker());manifest=json.loads((root/'extension.json').read_text());v.validate(manifest)
for name,change in [('runtime',lambda m:m.update(runtime='javascript')),('identity',lambda m:m.update(id='com.tezbar.fontawesome')),('protocol',lambda m:m.update(protocolVersion=2)),('artifact',lambda m:m.update(artifacts={'aarch64-apple-darwin':{'url':'https://example.invalid/fake'}})),('unknown-field',lambda m:m.update(origin={})),('refresh',lambda m:m['commands'][0].update(refreshMs=1))]:
 bad=copy.deepcopy(manifest);change(bad);assert list(v.iter_errors(bad)),name
assert manifest['replaces']==['raycast.fontawesome','com.tezbar.fontawesome'];assert manifest['commands'][0]['name']=='index';assert manifest['artifacts']=={}
print('Pinned schema: source manifest valid; runtime, identity, protocol, unpinned artifact, unknown origin and refresh negatives rejected.')
