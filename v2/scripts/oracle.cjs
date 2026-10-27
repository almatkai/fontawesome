// Execute pinned original TS/TSX with effect producers injected. No product endpoints or OS effects.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..'),source=path.join(root,'fixtures/source');
const {transformSync}=require(path.join(root,'qa/v1/node_modules/esbuild'));
let preferences={},stored,fetchData,fetchError,captured=[],clips=[],hud=[],fetches=[];
const jsx=(type,props,...children)=>({type,props:{...props,children:children.flat(Infinity).filter(x=>x!==false&&x!==null&&x!==undefined)}});
const named=name=>Object.assign(name,{ });
const Grid=Object.assign(()=>{},Object.fromEntries(['Item','EmptyView','Dropdown'].map(k=>[k,Object.assign(()=>{}, {Item:'List.Dropdown.Item',Section:'List.Dropdown.Section'})])));
Grid.Inset={Large:'large'};
const api={getPreferenceValues:()=>preferences,Clipboard:{copy:async x=>clips.push(x)},showHUD:async x=>hud.push(x),Action:'Action',ActionPanel:'ActionPanel',Grid,Color:{PrimaryText:'primary',SecondaryText:'secondary'},Icon:{ExclamationMark:'warning'},openExtensionPreferences:()=>{throw Error('OS effect blocked')}};
const utils={useCachedState:(key,defaultValue)=>[stored===undefined?defaultValue:stored,()=>{}],useLocalStorage:()=>({value:100,isLoading:false,setValue:()=>{}}),useFetch:(url,opts)=>{fetches.push({url,opts});return {data:fetchData,error:fetchError,isLoading:false,revalidate:()=>{throw Error('network blocked')}}}};
const cache=new Map();
function load(file){file=path.resolve(file);if(cache.has(file))return cache.get(file);const exports={};const module={exports};cache.set(file,exports);const code=transformSync(fs.readFileSync(file,'utf8'),{loader:file.endsWith('tsx')?'tsx':'ts',format:'cjs',jsxFactory:'__jsx',jsxFragment:'__Fragment'}).code;
const requireMock=id=>{if(id==='@raycast/api')return api;if(id==='@raycast/utils')return utils;if(id==='react')return {useMemo:f=>f(),useState:v=>[v,()=>{}]};if(id==='node:crypto')return require('node:crypto');if(id.startsWith('@/')||id.startsWith('.')){let p=id.startsWith('@/')?path.join(source,'src',id.slice(2)):path.resolve(path.dirname(file),id);p+=fs.existsSync(p+'.tsx')?'.tsx':'.ts';return load(p);}throw Error('Producer blocked: '+id);};
vm.runInNewContext(code,{exports,module,require:requireMock,__jsx:jsx,__Fragment:'Fragment',Buffer,Date,console:{log:()=>{throw Error('logging blocked')}}},{filename:file,timeout:1000});cache.set(file,module.exports);return module.exports;}
const util=name=>load(path.join(source,'src/utils',name+'.ts'));
const data=util('data'),actions=util('actions'),kitUtils=util('kits'),access=util('access-token'),queries=util('query');
async function main(){const output={styles:Object.entries(data.familyStylesByPrefix),queries:[],copies:[],filters:[],refresh:[],preferences:[],kitNormalization:[],actionOrders:[]};
for(const [prefix,style] of output.styles)output.queries.push({prefix,style,query:'hello world',body:queries.iconQuery('hello world',style)});
for(const [prefix] of [...output.styles,['kit']])for(const unicode of ['f007','1f600','nothex','0x41','-1','41tail','ffffffffffffffff','10000000000000001']){
const icon={id:'sample-icon',unicode,svgs:[{html:'<svg viewBox="0 0 1 1"><path d="M0 0L1 1"/><path d="M1 0"/></svg>',familyStyle:{prefix}}]};
for(const [id,fn] of [['copyIconName','copyFASlugToClipboard'],['copyIconClasses','copyFAClassesToClipboard'],['copyAsSvg','copySvgToClipboard'],['copyIconGlyph','copyFAGlyphToClipboard'],['copyIconUnicode','copyFAUnicodeClipboard']]){await actions[fn](icon);output.copies.push({icon,action:id,text:clips.pop(),hud:hud.pop()});}}
const kits=[{name:'Personal Icons',token:'local-kit-a'},{name:'Other',token:'local-kit-b'}];
for(const filter of ['', ' Personal Icons ', 'LOCAL-KIT-B',', ,', 'Personal','Personal Icons, LOCAL-KIT-B'])output.filters.push({kits,filter,result:kitUtils.filterKits(kits,filter)});
for(const arg of [{accessToken:'',now:100},{accessToken:'local',tokenTimeStart:100,now:3599999},{accessToken:'local',tokenTimeStart:100,now:3600100},{accessToken:'local',tokenTimeStart:0,now:5},{accessToken:'local',tokenTimeStart:1000,now:5}])output.refresh.push({arg,result:access.shouldRefreshAccessToken(arg)});
assert.equal(access.getApiTokenConfigurationError(['kits_read'],false),undefined);assert.ok(access.getApiTokenConfigurationError(['kits_read'],true));assert.equal(access.getApiTokenConfigurationError(['kits_read','domains_read','svg_icons_pro'],true),undefined);
for(const custom of [false,true]){preferences={API_TOKEN:custom?'local-oracle-only':'',STYLE_PREFERENCE:'fat',REMEMBER_LAST_KIT:true,KIT_FILTER:'Other'};const result=load(path.join(source,'src/hooks/usePreferences.ts')).usePreferences();assert.equal(result.hasCustomToken,custom);assert.equal(result.account,custom?'pro':'free');assert.equal(result.STYLE_PREFERENCE,custom?'fat':'fas');output.preferences.push({custom,style:result.STYLE_PREFERENCE,account:result.account});}
const uploads=[{name:'Alpha',unicode:'f010',width:16,height:16,pathData:['M0 0L1 1','M1 0']},{name:'Beta',unicode:65,width:16,height:16,pathData:'M0 0L2 2'}];
for(const query of ['', ' alpha ', '65','zzz']){stored={data:{me:{kits:[{...kits[0],iconUploads:uploads}]}}};fetchData=undefined;
const normalized=load(path.join(source,'src/hooks/useData.ts')).useData('local-access-only','local-scope',true,query,'kit:local-kit-a').data;output.kitNormalization.push({query,kits:[{...kits[0],iconUploads:uploads}],selected:'local-kit-a',result:normalized});}
const example=output.copies[0].icon;
for(const primary of ['copyIconName','copyIconClasses','copyAsSvg','copyIconGlyph','copyIconUnicode','unknown']){preferences={PRIMARY_ACTION:primary};const rendered=load(path.join(source,'src/components/IconActions.tsx')).IconActions({searchItem:example});const order=rendered.props.children.map(n=>n.props.key);output.actionOrders.push({primary,order});}
// Run actual hooks and command with injected producers. Default producers never perform effects.
stored=undefined;fetchData={access_token:'local-access-only',expires_in:3600,scopes:[]};fetchError=undefined;preferences={API_TOKEN:'',STYLE_PREFERENCE:'fas',PRIMARY_ACTION:'copyIconName'};
const tokenHook=load(path.join(source,'src/hooks/useAccessToken.ts')).useAccessToken('local-source-only',false);assert.equal(tokenHook.tokenError,undefined);
fetchError=Error('injected');const failure=load(path.join(source,'src/hooks/useAccessToken.ts')).useAccessToken('local-source-only',true);assert.equal(failure.tokenError.title,'Font Awesome API token is invalid');
fetchError=undefined;fetchData={data:{search:[example]}};load(path.join(source,'src/index.tsx')).default();
// Demonstrate the source query interpolation defect without contacting a service.
assert.ok(queries.iconQuery('" } injected { "','Classic, Solid').includes('query:"" } injected { ""'));
fs.writeFileSync(path.join(root,'fixtures/oracle.json'),JSON.stringify(output,null,2)+'\n');console.log('Original TS/TSX oracle: '+output.copies.length+' copy cases, '+output.queries.length+' styles; hooks/preferences/index executed with blocked OS/network producers.');}
main().catch(e=>{console.error(e.message);process.exitCode=1});
