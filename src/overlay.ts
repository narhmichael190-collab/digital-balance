import { invoke } from '@tauri-apps/api/core';

const params = new URLSearchParams(window.location.search);
const type = params.get('type') ?? 'aura';
const phase = params.get('phase') ?? 'start';
const accent = params.get('accent') ?? '#60a5fa';
const appName = params.get('app') ?? 'Protected app';
const targetExe = params.get('exe') ?? appName;
const pid = Number(params.get('pid') ?? '0');
const reopen = Number(params.get('reopen') ?? '1');

const root = document.getElementById('root')!;
const style = document.createElement('style');
style.textContent = `
:root{font-family:"Open Sans","Segoe UI",sans-serif;color:#eef4fb;background:rgba(4,7,12,.78)}*{box-sizing:border-box}html,body,#root{margin:0;width:100%;height:100%;overflow:hidden}body{display:grid;place-items:center;background:transparent;backdrop-filter:blur(12px)}.scene{position:relative;width:100%;height:100%;display:grid;place-items:center;background:radial-gradient(circle at 50% 50%,${accent}26,transparent 28%),radial-gradient(circle at 50% 50%,rgba(255,255,255,.06),transparent 50%)}.orb{position:absolute;width:min(56vw,640px);aspect-ratio:1;border-radius:50%;border:1px solid ${accent}66;box-shadow:0 0 90px ${accent}33,inset 0 0 80px ${accent}18;animation:spin 6s linear infinite}.orb::before,.orb::after{content:"";position:absolute;inset:9%;border:1px solid ${accent}44;border-radius:50%;animation:spin 9s linear reverse infinite}.orb::after{inset:19%;border-style:dashed;animation-duration:13s}.card{position:relative;z-index:3;width:min(620px,78vw);padding:34px;border:1px solid ${accent}55;border-radius:22px;background:rgba(8,12,18,.88);box-shadow:0 30px 120px rgba(0,0,0,.55);text-align:center}.eyebrow{font-size:11px;letter-spacing:.2em;text-transform:uppercase;color:${accent};font-weight:800}.title{font-size:38px;font-weight:900;letter-spacing:-.04em;margin-top:10px}.body{font-size:14px;line-height:1.7;color:#aebacc;margin-top:10px}.button{margin-top:22px;border:1px solid ${accent}66;background:${accent}1c;color:#fff;padding:12px 18px;border-radius:11px;font-weight:800;cursor:pointer}.aura-flash{animation:flash 1.2s ease-out}.hint{margin-top:14px;font-size:11px;color:#7f8da1}@keyframes spin{to{transform:rotate(360deg)}}@keyframes flash{0%,100%{opacity:.15;filter:brightness(.7)}22%{opacity:1;filter:brightness(1.45)}45%{opacity:.45}70%{opacity:.92}}
`;
document.head.appendChild(style);

function closeOverlay(){ invoke('close_policy_overlay').catch(()=>window.close()); }
function closeTarget(){ invoke('force_close_policy_target', { pid, exe: targetExe }).then(() => closeOverlay()).catch(() => undefined); }

if(type === 'limit'){
  root.innerHTML = `<div class="scene"><div class="orb"></div><div class="card"><div class="eyebrow">Digital Balance · distraction guard</div><div class="title">${appName} is outside your limit.</div><div class="body">This app has reached its protection limit. Close it to continue. Warning ${reopen} of 3.</div><button class="button" id="close">Close ${appName}</button><div class="hint">The warning stays active until the protected app closes.</div></div></div>`;
  document.getElementById('close')?.addEventListener('click', closeTarget);
} else {
  root.innerHTML = `<div class="scene aura-flash"><div class="orb"></div><div class="card"><div class="eyebrow">Digital Balance</div><div class="title">${phase==='end'?'Focus complete.':'Focus activated.'}</div><div class="body">${phase==='end'?'Session secured. Your rewards are waiting.':'The session is live. Protect the next block of time.'}</div></div></div>`;
}
