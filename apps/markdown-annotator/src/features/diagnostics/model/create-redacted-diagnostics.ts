export type DiagnosticInput={appVersion:string;commit:string;platform:string;errorCodes:string[];rootCount:number;sessionCount:number};
export function createRedactedDiagnostics(input:DiagnosticInput){return JSON.stringify({schemaVersion:1,createdAt:new Date().toISOString(),...input},null,2)}
