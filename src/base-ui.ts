import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';

type Field = {name:string;kind:string;required:boolean};
type Table = {name:string;count:number;fields:Field[]};
type Query = {columns:string[];rows:unknown[][];truncated:boolean};
type IndexInfo = {name:string;unique:boolean;columns:string[]};
const html = (s:unknown) => String(s ?? '').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const byId = (id:string)=>document.getElementById(id)!;

export class BaseUI {
  path:string|null=null;
  table:string|null=null;
  tables:Table[]=[];
  results:Query|null=null;
  query='';
  offset=0;
  currentSearch='';
  indexes:IndexInfo[]=[];
  private busy=false;
  constructor(private notice:(message:string)=>void, private updateHeader:()=>void, private toSheets:(table:string)=>Promise<void>){}
  async create(){const p=await save({defaultPath:'Database.kdb',filters:[{name:'Koffice Base',extensions:['kdb']}]});if(!p)return;await invoke('base_create',{path:p});this.path=p;this.table=null;this.results=null;this.offset=0;await this.load();}
  async open(){const p=await open({multiple:false,filters:[{name:'Koffice Base',extensions:['kdb']}]});if(typeof p!=='string')return;await invoke('base_open',{path:p});this.path=p;this.table=null;this.results=null;this.offset=0;await this.load();}
  async initialize(){this.path=await invoke<string|null>('base_path');if(this.path)await this.load();else this.render();}
  async load(){this.tables=await invoke<Table[]>('base_tables');if(!this.tables.some(t=>t.name===this.table))this.table=this.tables[0]?.name||null;if(this.table){this.results=await invoke<Query>('base_browse_page',{table:this.table,search:this.currentSearch||null,offset:this.offset});this.indexes=await invoke<IndexInfo[]>('base_indexes',{table:this.table});}else{this.results=null;this.indexes=[];}this.updateHeader();this.render();}
  async browse(search:string){if(!this.table)return;this.offset=0;this.currentSearch=search;await this.load();}
  async sql(){const sql=(byId('base-sql') as HTMLTextAreaElement).value;this.query=sql;this.results=await invoke<Query>('base_select',{sql});this.render();}
  exportCsv(){if(!this.results){this.notice('No query results to export');return;}const r=this.results;
    const value=(v:unknown)=>'"'+String(v??'').replace(/"/g,'""')+'"';
    const output='\ufeff'+[r.columns.map(value).join(','),...r.rows.map(row=>row.map(value).join(','))].join('\r\n');
    const url=URL.createObjectURL(new Blob([output],{type:'text/csv;charset=utf-8'}));const a=document.createElement('a');a.href=url;a.download=(this.table??'query')+'.csv';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
    if(r.truncated)this.notice('CSV contains only the first 500 returned rows');
  }
  private downloadCsv(content:string,name:string){
    const url=URL.createObjectURL(new Blob(['\ufeff',content],{type:'text/csv;charset=utf-8'}));
    const a=document.createElement('a');a.href=url;a.download=name;a.click();
    setTimeout(()=>URL.revokeObjectURL(url),1000);
  }
  async exportEntireTable(){
    if(!this.table)return;
    const csv=await invoke<string>('base_export_table_csv',{table:this.table});
    this.downloadCsv(csv,`${this.table}.csv`);
  }
  async backup(){
    if(!this.path)return;
    const destination=await save({defaultPath:'Koffice-backup.kdb',filters:[{name:'Koffice Base',extensions:['kdb']}]});
    if(!destination)return;
    await invoke('base_backup',{destination});this.notice('Database backup created');
  }
  async checkIntegrity(){
    const result=await invoke<string>('base_integrity_check');
    this.notice(result==='ok'?'SQLite database integrity: OK':`Integrity check: ${result}`);
  }
  async showReport(){
    if(!this.table)return;
    const table=this.tables.find(t=>t.name===this.table);
    if(!table)return;
    const groupField=window.prompt(`Group by field: ${table.fields.map(f=>f.name).join(', ')}`,table.fields.find(f=>f.name!=='id')?.name??'id');
    if(!groupField)return;
    const metric=window.prompt('Numeric field for SUM and AVG (leave blank for COUNT only)','');
    if(metric===null)return;
    this.results=await invoke<Query>('base_group_report',{table:this.table,groupField,metricField:metric||null});
    this.query=`Report: ${groupField}${metric?` / ${metric}`:''}`;
    this.render();
  }
  async importCsv(file:File){
    if(!this.table)return;
    if(file.size>5_000_000)throw Error('CSV exceeds 5 MB');
    const content=await file.text();
    const result=await invoke<{inserted:number}>('base_import_csv',{table:this.table,content});
    this.offset=0;await this.load();this.notice(`Imported ${result.inserted} records`);
  }
  async indexCreate(){if(!this.table)return;
    const raw=window.prompt('Column names for index (comma separated)','name');if(!raw)return;
    const columns=raw.split(',').map(v=>v.trim());
    const unique=window.confirm('Require unique values? OK = UNIQUE, Cancel = ordinary index');
    await invoke('base_create_index',{table:this.table,columns,unique});await this.load();this.notice('Index created');
  }
  async indexDelete(name:string){if(!window.confirm(`Delete index ${name}?`))return;
    await invoke('base_delete_index',{name});await this.load();this.notice('Index removed');
  }
  async nextPage(direction:number){const offset=Math.max(0,this.offset+direction*500);if(offset===this.offset)return;this.offset=offset;await this.load();}
  async sendToSheets(){if(!this.table)return;
    await this.toSheets(this.table);
  }
  private async createTable(){const name=window.prompt('Table name (letters, numbers, underscores)','contacts');if(!name)return;
    const raw=window.prompt('Fields: name:TEXT, age:INTEGER, active:INTEGER','name:TEXT, email:TEXT');if(!raw)return;
    const fields=raw.split(',').map(part=>{const [name,kind='TEXT']=part.trim().split(':');return {name:name.trim(),kind:kind.trim().toUpperCase(),required:false};});
    await invoke('base_create_table',{table:name,fields});this.table=name;await this.load();this.notice(`Table ${name} created`);
  }
  private async addRow(){const t=this.tables.find(x=>x.name===this.table);if(!t)return;
    const values:Record<string,string|number|null>={};for(const f of t.fields.filter(f=>f.name!=='id')){const v=window.prompt(`${f.name} (${f.kind})`, '');if(v===null)return;values[f.name]=v===''?null:(['INTEGER','REAL'].includes(f.kind.toUpperCase())?Number(v):v);if(typeof values[f.name]==='number'&&!Number.isFinite(values[f.name]))throw Error('Invalid number');}
    await invoke('base_insert',{table:this.table,values});await this.load();this.notice('Record added');
  }
  private async editRow(id:number){const t=this.tables.find(x=>x.name===this.table);if(!t||!this.results)return;const idx=this.results.columns.indexOf('id');const record=this.results.rows.find(row=>Number(row[idx])===id);if(!record)return;
    const values:Record<string,unknown>={};for(const f of t.fields.filter(f=>f.name!=='id')){const col=this.results.columns.indexOf(f.name);const current=record[col];const v=window.prompt(`Edit ${f.name}`,current===null?'':String(current));if(v===null)return;values[f.name]=v===''?null:(['INTEGER','REAL'].includes(f.kind.toUpperCase())?Number(v):v);if(typeof values[f.name]==='number'&&!Number.isFinite(values[f.name]))throw Error('Invalid number');}
    await invoke('base_update',{table:this.table,id,values});await this.load();this.notice('Record updated');
  }
  private async deleteRow(id:number){if(!this.table||!window.confirm(`Permanently delete record #${id}?`))return;await invoke('base_delete',{table:this.table,id});await this.load();this.notice('Record deleted');}
  private async task(fn:()=>Promise<void>){if(this.busy)return;this.busy=true;try{await fn();}catch(e){this.notice(String(e));}finally{this.busy=false;}}
  render(){const root=byId('canvas');if(!this.path){root.innerHTML=`<div class="base-empty"><h2>Koffice Base</h2><p>Create or open a .kdb database to manage structured information, records, and read-only SQL reports.</p><button id="base-new">＋ Create database</button><button id="base-open">Open database</button></div>`;
      byId('base-new').onclick=()=>void this.task(()=>this.create());byId('base-open').onclick=()=>void this.task(()=>this.open());return;}
    const r=this.results;const idCol=r?.columns.indexOf('id')??-1;
    root.innerHTML=`<div class="base-app"><aside class="base-tables"><div class="base-group">DATABASE TABLES</div>${this.tables.map(t=>`<button class="base-table ${this.table===t.name?'current':''}" data-table="${html(t.name)}">${html(t.name)} <small>${t.count}</small></button>`).join('')}<button class="base-create" id="base-create-table">＋ New table</button><div class="base-meta">${html(this.path)}</div></aside><div class="base-work"><header class="base-actions"><b>${html(this.table??'Database')}</b><span>${this.tables.length} tables</span><button id="base-add" ${this.table?'':'disabled'}>＋ Record</button><input id="base-search" placeholder="Search text columns" aria-label="Search rows" ${this.table?'':'disabled'}><button id="base-search-go">Search</button><button id="base-reload">Reload</button><button id="base-import">Import CSV</button><input id="base-csv-file" type="file" accept=".csv,text/csv" hidden><button id="base-to-sheets">To Sheets</button><button id="base-export-all">Export table CSV</button><button id="base-report">Group report</button><button id="base-backup">Backup</button><button id="base-check">Check DB</button><button id="base-index">＋ Index</button></header><div class="base-grid">${r?`<table><thead><tr>${r.columns.map(c=>`<th>${html(c)}</th>`).join('')}<th>Actions</th></tr></thead><tbody>${r.rows.map(row=>`<tr>${row.map(v=>`<td title="${html(v)}">${html(v)}</td>`).join('')}<td>${idCol<0?'':`<button data-edit="${Number(row[idCol])}">Edit</button><button data-delete="${Number(row[idCol])}">Delete</button>`}</td></tr>`).join('')}</tbody></table>`:'<p>Select a table or write a query.</p>'}</div><div class="base-footer">${r?`${this.offset+1}–${this.offset+r.rows.length} rows${r.truncated?' · More available':''}`:'No result'} <button id="base-prev" ${this.offset===0?'disabled':''}>← Previous</button><button id="base-next" ${!r?.truncated?'disabled':''}>Next →</button></div><div class="base-indexes"><strong>Indexes</strong> ${this.indexes.map(i=>`<span>${html(i.name)} (${html(i.columns.join(', '))}) ${i.unique?'UNIQUE':''} <button data-drop-index="${html(i.name)}">×</button></span>`).join('')||'<small>No indexes</small>'}</div><div class="base-query"><label for="base-sql">SQL Explorer <small>read-only SELECT / WITH, up to 500 rows</small></label><textarea id="base-sql" spellcheck="false" placeholder="SELECT * FROM contacts LIMIT 20">${html(this.query)}</textarea><button id="base-run-sql">Run query</button></div></div></div>`;
    document.querySelectorAll<HTMLButtonElement>('[data-table]').forEach(b=>b.onclick=()=>void this.task(async()=>{this.table=b.dataset.table!;this.query='';this.offset=0;this.currentSearch='';await this.load();}));
    byId('base-create-table').onclick=()=>void this.task(()=>this.createTable());byId('base-add').onclick=()=>void this.task(()=>this.addRow());byId('base-reload').onclick=()=>void this.task(()=>this.load());byId('base-run-sql').onclick=()=>void this.task(()=>this.sql());
    byId('base-import').onclick=()=>byId('base-csv-file').click();
    (byId('base-csv-file') as HTMLInputElement).onchange=e=>{const file=(e.target as HTMLInputElement).files?.[0];if(file)void this.task(()=>this.importCsv(file));};
    byId('base-to-sheets').onclick=()=>void this.task(()=>this.sendToSheets());
    byId('base-export-all').onclick=()=>void this.task(()=>this.exportEntireTable());
    byId('base-report').onclick=()=>void this.task(()=>this.showReport());
    byId('base-backup').onclick=()=>void this.task(()=>this.backup());
    byId('base-check').onclick=()=>void this.task(()=>this.checkIntegrity());
    byId('base-index').onclick=()=>void this.task(()=>this.indexCreate());
    byId('base-prev').onclick=()=>void this.task(()=>this.nextPage(-1));
    byId('base-next').onclick=()=>void this.task(()=>this.nextPage(1));
    document.querySelectorAll<HTMLButtonElement>('[data-drop-index]').forEach(b=>b.onclick=()=>void this.task(()=>this.indexDelete(b.dataset.dropIndex!)));
    byId('base-search-go').onclick=()=>void this.task(()=>this.browse((byId('base-search') as HTMLInputElement).value));
    document.querySelectorAll<HTMLButtonElement>('[data-edit]').forEach(b=>b.onclick=()=>void this.task(()=>this.editRow(Number(b.dataset.edit))));
    document.querySelectorAll<HTMLButtonElement>('[data-delete]').forEach(b=>b.onclick=()=>void this.task(()=>this.deleteRow(Number(b.dataset.delete))));
  }
}