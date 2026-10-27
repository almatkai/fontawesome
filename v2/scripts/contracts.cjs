const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),host=path.join(root,'fixtures/host');
const {transformSync}=require(path.join(root,'qa/v1/node_modules/esbuild'));
const jsx=(type,props,...children)=>({type,props:{...props,children:children.flat(Infinity).filter(x=>x!==false&&x!==null&&x!==undefined)}});
let slots=[],index=0,effects=[];const react={useMemo:f=>f(),useRef:initial=>({current:initial}),useEffect:f=>effects.push(f),useState:initial=>{const slot=index++;if(!(slot in slots))slots[slot]=initial;return [slots[slot],v=>{slots[slot]=typeof v==='function'?v(slots[slot]):v}];}};
function load(relative,extra=''){const file=path.join(host,relative);const module={exports:{}};const code=transformSync(fs.readFileSync(file,'utf8')+extra,{loader:relative.endsWith('tsx')?'tsx':'ts',format:'cjs',jsxFactory:'__jsx',jsxFragment:'__Fragment'}).code;
const mock=id=>{if(id==='react')return react;if(id.endsWith('/ui/primitives'))return {Hint:'Hint',HintBar:'HintBar',Kbd:'Kbd',cx:(...x)=>x.filter(Boolean).join(' ')};if(id==='./grid-content')return load('src/renderer/src/raycast-api/grid-content.ts');throw Error('Unapproved module '+id)};
vm.runInNewContext(code,{module,exports:module.exports,require:mock,__jsx:jsx,__Fragment:'Fragment',setTimeout:()=>{throw Error('timer blocked')},clearTimeout:()=>{}},{timeout:1000});return module.exports;}
const grid=load('src/renderer/src/raycast-api/grid-runtime.tsx','\nexport { collectGridItems, parseGridAccessory };');
const view=JSON.parse(fs.readFileSync(path.join(root,'qa/renderer-view.json'),'utf8'));assert.equal(view.root.type,'Grid');assert.equal(view.refreshMs,null);
const items=grid.collectGridItems(view.root);assert.equal(items.length,1);assert.ok(items[0].image.startsWith('data:image/svg+xml;base64,'));
const primary=view.actions.find(a=>a.id===items[0].actionIds[0]);assert.equal(primary.title,'Copy as SVG');
assert.ok(items[0].actionIds.some(id=>view.actions.find(a=>a.id===id)?.title==='Retry Font Awesome Request'));
const accessory=grid.parseGridAccessory(view.root.props.searchBarAccessory);assert.equal(accessory.value,'fat');assert.equal(accessory.options.length,18);
let invoked=[];const props={root:view.root,title:view.title,actions:view.actions,onBack:()=>{},onRunPrimaryAction:(...a)=>invoked.push(a),onOpenActions:(...a)=>invoked.push(a),onSearchTextChanged:()=>{throw Error('network producer blocked')}};
function render(){index=0;return grid.GridRuntime(props);}let tree=render();const initialEffects=effects;effects=[];for(const f of initialEffects)f();tree=render();
function all(node){if(!node||typeof node!=='object')return [];return [node,...(node.props?.children??[]).flatMap(all)];}
let nodes=all(tree);const select=nodes.find(n=>n.type==='select');assert.equal(select.props.value,'fat');const kit=accessory.options[17].value;select.props.onChange({target:{value:kit}});assert.equal(invoked[0][0],accessory.actionId);assert.equal(invoked[0][1].value,kit);
const item=nodes.find(n=>n.type==='button'&&n.props.key==='row:0');item.props.onClick();assert.equal(invoked[1][0],items[0].actionIds[0]);assert.ok(nodes.find(n=>n.props?.style?.gridTemplateColumns==='repeat(8, minmax(0, 1fr))'));
const error=grid.gridEmptyState({type:'Grid',props:{},children:[{type:'Grid.EmptyView',props:{title:'Request failed',description:'Use Retry'}}]},'');assert.equal(error.title,'Request failed');
const image=load('src/renderer/extensionImages.ts');assert.equal(image.extensionImageSource(items[0].image),items[0].image);
console.log('Pinned renderer: actual adapter Grid, 8 columns, controlled style/kit, primary action, reachable Retry, inert data image and empty error contract passed (mock React; no GUI).');
