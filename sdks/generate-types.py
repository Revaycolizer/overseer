"""Synchronize shared REST response models from server DTOs; --check detects drift.

This deliberately supports a bounded set of flat Rust DTOs, not arbitrary Rust.
Unsupported field types fail rather than silently producing an incorrect contract.
"""
from pathlib import Path
import re,sys
ROOT=Path(__file__).resolve().parents[1]
SOURCES={
 'src/server/metrics.rs':['ServiceHealth'],
 'src/backend/conversations/crud.rs':['Conversation'],
 'src/backend/conversations/search.rs':['ConversationSearchResult'],
 'src/backend/messages/types.rs':['Message','Attachment'],
 'src/engine/utils/discovery.rs':['AgentInfo','ToolInfo'],
 'src/db/mod.rs':['Supervisor','Planner'],
 'src/backend/workflows/models.rs':['Workflow','WorkflowStep','WorkflowRun','WorkflowStepRun'],
 'src/backend/work_items/types.rs':['WorkItem','WorkItemEvent'],
 'src/backend/artifacts/queries.rs':['Artifact'],
 'src/backend/version_control/queries.rs':['EntityVersion'],
 'src/backend/health/mod.rs':['RuntimeHealth'],
 'src/backend/system/load.rs':['SystemLoad'],
 'src/backend/registry.rs':['RuntimeRegistryStatus'],
 'src/backend/computer_supervision/state.rs':['ComputerSupervisionState'],
}
def convert(t,key,lang):
 if t.startswith('Option<'):
  inner=convert(t[7:-1],key,lang);return {'ts':f'{inner} | null','py':f'Optional[{inner}]','rs':f'Option<{inner}>'}[lang]
 if t.startswith('Vec<'):
  inner=convert(t[4:-1],key,lang);return {'ts':f'{inner}[]','py':f'list[{inner}]','rs':f'Vec<{inner}>'}[lang]
 if t in ('String','chrono::NaiveDateTime'):
  return {'ts':'string','py':'datetime' if t.startswith('chrono') else 'str','rs':'String'}[lang]
 if t=='bool':return {'ts':'boolean','py':'bool','rs':'bool'}[lang]
 if t in ('i64','u64','i32','u32','f64','f32'):
  return {'ts':'ApiId' if key=='id' or key.endswith('_id') else 'number','py':'float' if t.startswith('f') else 'int','rs':t}[lang]
 if t in ('Attachment',):return t
 raise ValueError(f'Unsupported field {key}: {t}')
paths={'ts':'sdks/typescript/src/types.ts','py':'sdks/python/src/overseer/models.py','rs':'crates/overseer-client/src/types.rs'}
contents={k:(ROOT/v).read_text() for k,v in paths.items()}
for source,names in SOURCES.items():
 text=(ROOT/source).read_text()
 for name in names:
  match=re.search(r'(?:pub )?struct '+('HealthResponse' if name=='ServiceHealth' else name)+r'\s*\{(.*?)\n\}',text,re.S)
  assert match,name
  fields=re.findall(r'(?:pub )?(\w+): ([^,\n]+)',match[1])
  for lang,content in contents.items():
   if lang=='ts':
    block='export interface '+name+' {\n'+''.join(f'  {key}: {convert(t,key,lang)};\n' for key,t in fields)+'}'
    pattern=r'export interface '+name+r' \{.*?\n\}'
   elif lang=='py':
    block='class '+name+'(BaseModel):\n'+''.join(f'    {key}: {convert(t,key,lang)}'+(' = None' if t.startswith('Option<') else ' = Field(default_factory=list)' if key=='attachments' else '')+'\n' for key,t in fields)
    pattern=r'class '+name+r'\(BaseModel\):.*?(?=\n\n(?:class |#)|\Z)'
   else:
    block='pub struct '+name+' {\n'+''.join(('    #[serde(default)]\n' if key=='attachments' else '')+f'    pub {key}: {convert(t,key,lang)},\n' for key,t in fields)+'}'
    pattern=r'pub struct '+name+r' \{.*?\n\}'
   content,count=re.subn(pattern,lambda _:block,content,flags=re.S);assert count==1,(lang,name,count)
   contents[lang]=content
contents['ts']=re.sub(r'(\b(?:id|[a-z_]+_id)): number',r'\1: ApiId',contents['ts'])
if 'export type ApiId' not in contents['ts']:contents['ts']='/** Exact decimal record ID returned by the SDK. */\nexport type ApiId = string;\n/** Accepted input: decimal string, bigint, or a safe integer. */\nexport type IdInput = string | bigint | number;\n\n'+contents['ts']
changed=[]
for lang,path in paths.items():
 if contents[lang]!=(ROOT/path).read_text():
  changed.append(path)
  if '--check' not in sys.argv:(ROOT/path).write_text(contents[lang])
if '--check' in sys.argv and changed:raise SystemExit('Outdated SDK models: '+', '.join(changed))
print('SDK response models match server DTOs.')
