import { bench, describe } from "vitest";
import { createFileBrowserRows } from "./create-file-browser-rows";
const entries=Array.from({length:10_000},(_,index)=>({path:`group-${Math.floor(index/100)}/file-${index}.md`,kind:"file" as const,size:index}));
describe("SC-002~SC-004 file browser scale",()=>{bench("10,000 entries natural sort/search/ancestor rows",()=>{createFileBrowserRows(entries,{expandedPaths:new Set(),searchQuery:"file-99",sort:{by:"name",direction:"asc"},compressSingleDirectoryChains:true});});});
