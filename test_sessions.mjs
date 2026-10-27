import { readFileSync } from 'node:fs';

// --- fakes -------------------------------------------------------------
const store = new Map();
globalThis.localStorage = {
  getItem: k => (store.has(k) ? store.get(k) : null),
  setItem: (k, v) => void store.set(k, String(v)),
  removeItem: k => void store.delete(k),
};
globalThis.window = { addEventListener() {} };
globalThis.document = { addEventListener() {}, hidden: false };

// zustand clone, same shape as Liquid.js `g`
function mk(fn) {
  let st, subs = new Set();
  const set = (p, rep) => {
    const n = typeof p === 'function' ? p(st) : p;
    if (!Object.is(n, st)) {
      const prev = st;
      st = rep ?? (typeof n !== 'object' || !n) ? n : Object.assign({}, st, n);
      subs.forEach(s => s(st, prev));
    }
  };
  const get = () => st;
  const api = { setState: set, getState: get, getInitialState: () => init, subscribe: s => (subs.add(s), () => subs.delete(s)) };
  const init = (st = fn(set, get, api));
  return api;
}

let chatSetCalls = [];
const n = { chatSet: j => (chatSetCalls.push(JSON.parse(j)), Promise.resolve()) };

let persisted = [];
const M = mk((set, get) => ({
  loaded: false, messages: [], streaming: false, draft: '', pending: [], attaching: 0, notice: null,
  load: () => Promise.resolve().then(() => set({ loaded: true })),
  reset: () => get().load().then(() => { persisted = []; set({ messages: [], streaming: false, pending: [], notice: null }); }),
  say: t => set({ notice: t }),
}));

// --- load the module under test ---------------------------------------
const src = readFileSync(new URL('./session_store.js.template', import.meta.url), 'utf8');
new Function('M', 'n', src)(M, n);

const S = globalThis.window.__sessions__;
const msg = (role, text, id) => ({ id: id || role[0] + Math.random().toString(36).slice(2, 6), role, text });
const ok = (c, label) => { if (!c) { console.error('FAIL ' + label); process.exitCode = 1; } else console.log('ok   ' + label); };
const tick = () => new Promise(r => setTimeout(r, 0));

// --- 1. save mints a session, title from first user message -----------
M.setState({ messages: [msg('user', '  Como faço  ESP   no Rust?  '), msg('assistant', 'assim e assim')] });
const id1 = S.save();
ok(id1, 'save returns an id');
let l = S.list();
ok(l.length === 1, 'one session listed');
ok(l[0].title === 'Como faço ESP no Rust?', 'title normalised: ' + JSON.stringify(l[0].title));
ok(l[0].n === 2, 'message count 2');
ok(/\d/.test(l[0].when), 'when rendered: ' + l[0].when);

// --- 2. save again upserts the same session, no duplicate -------------
M.setState({ messages: [...M.getState().messages, msg('user', 'e aimbot?')] });
const id2 = S.save();
ok(id2 === id1, 'same session id on re-save');
ok(S.list().length === 1, 'still one session (upsert, not append)');
ok(S.list()[0].n === 3, 'count grew to 3');

// --- 3. long title is cut on a word boundary --------------------------
store.clear();
M.setState({ messages: [msg('user', 'preciso de um overlay externo com imgui e d3d11 para o jogo inteiro')] });
S.save();
const t = S.list()[0].title;
ok(t.length <= 41 && t.endsWith('…') && !t.includes('  '), 'long title cut: ' + JSON.stringify(t));

// --- 4. errored and empty messages never get archived -----------------
store.clear();
M.setState({ messages: [msg('user', 'oi'), { id: 'e1', role: 'assistant', text: 'boom', error: true }, { id: 'e2', role: 'assistant', text: '   ' }] });
S.save();
ok(S.list()[0].n === 1, 'only the live message archived');

// --- 5. attachment-only message keeps the file names as its text ------
store.clear();
M.setState({ messages: [{ id: 'a1', role: 'user', text: '', attachments: [{ id: 'x', name: 'dump.txt' }, { id: 'y', name: 'shot.png' }] }] });
S.save();
ok(S.list()[0].title === 'dump.txt, shot.png', 'attachment names become the title: ' + S.list()[0].title);

// --- 6. intercepted reset archives, then clears, then a new id --------
store.clear();
M.setState({ messages: [msg('user', 'primeira conversa'), msg('assistant', 'resposta')] });
await M.getState().reset();
await tick();
ok(S.list().length === 1, 'reset archived the conversation');
ok(S.current() === null, 'reset cleared the current id');
ok(M.getState().messages.length === 0, 'reset still clears the store');

M.setState({ messages: [msg('user', 'segunda conversa')] });
const id3 = S.save();
ok(id3 !== S.list()[1].id, 'second conversation got its own id');
ok(S.list().length === 2, 'two sessions now');
ok(S.list()[0].title === 'segunda conversa', 'newest first');

// --- 7. restore puts the messages back and persists them --------------
const old = S.list()[1];
chatSetCalls = [];
const r = S.load(old.id);
ok(r && r.id === old.id, 'load found the session');
await tick(); await tick();
ok(M.getState().messages.length === 2, 'restored 2 messages');
ok(M.getState().messages[0].text === 'primeira conversa', 'restored content');
ok(M.getState().streaming === false && M.getState().pending.length === 0, 'restore resets transient state');
ok(chatSetCalls.length === 1 && chatSetCalls[0].length === 2, 'restore persisted via chatSet');
ok(S.current() === old.id, 'current id switched to the restored session');
ok(S.list().length === 2, 'restoring archived the open chat without duplicating');

// --- 8. rename sticks across a later save -----------------------------
S.rename(old.id, '  ESP do Rust  ');
ok(S.list().find(x => x.id === old.id).title === 'ESP do Rust', 'rename trimmed and stored');
M.setState({ messages: [...M.getState().messages, msg('user', 'mais uma pergunta')] });
S.save();
ok(S.list().find(x => x.id === old.id).title === 'ESP do Rust', 'rename survives autosave');

// --- 9. delete ---------------------------------------------------------
ok(S.delete(old.id) === true, 'delete reports success');
ok(S.list().length === 1, 'session gone');
ok(S.current() === null, 'deleting the open session cleared the current id');
ok(S.delete('nope') === false, 'deleting an unknown id is a no-op');

// --- 10. corrupt storage degrades to an empty list --------------------
store.set('islandSessions', '{not json');
ok(S.list().length === 0, 'corrupt storage -> empty list');
store.set('islandSessions', '[{"id":"a"},null,3]');
ok(S.list().length === 0, 'malformed rows dropped');

// --- 11. empty conversation is never archived -------------------------
store.clear();
M.setState({ messages: [] });
ok(S.save() === null, 'empty conversation not archived');
ok(S.list().length === 0, 'nothing listed');

// --- 12. the 40-session cap holds ------------------------------------
store.clear();
for (let i = 0; i < 45; i++) {
  M.setState({ messages: [msg('user', 'conversa ' + i)] });
  S.save();
  S.load.length; // no-op
  store.delete('islandSessionCur'); // force a fresh id each round
}
ok(S.list().length === 40, 'capped at 40, got ' + S.list().length);
ok(S.list()[0].title === 'conversa 44', 'newest kept');

// --- 13. label(): derived live, overridden by a rename ---------------
store.clear();
M.setState({ messages: [] });
ok(S.label(M.getState().messages) === '', 'label empty for an empty chat');
M.setState({ messages: [msg('user', 'como dumpar offsets do il2cpp')] });
ok(S.label(M.getState().messages) === 'como dumpar offsets do il2cpp', 'label derived before any save');
S.save();
ok(S.label(M.getState().messages) === 'como dumpar offsets do il2cpp', 'label still derived after save');
ok(S.renameCurrent('IL2CPP') === true, 'renameCurrent on the open session');
ok(S.label(M.getState().messages) === 'IL2CPP', 'label honours the rename');
M.setState({ messages: [...M.getState().messages, msg('user', 'outra pergunta')] });
S.save();
ok(S.label(M.getState().messages) === 'IL2CPP', 'rename survives further messages');
ok(S.renameCurrent('') === true, 'empty rename accepted');
ok(S.label(M.getState().messages) === 'como dumpar offsets do il2cpp', 'empty rename reverts to derived');

// --- 14. renameCurrent on an unsaved chat saves it first --------------
store.clear();
M.setState({ messages: [msg('user', 'primeira')] });
ok(S.current() === null, 'no session yet');
ok(S.renameCurrent('Batizada') === true, 'renameCurrent saves then renames');
ok(S.current() !== null, 'a session now exists');
ok(S.list()[0].title === 'Batizada', 'title stored');
ok(S.renameCurrent('x') === true, 'renameCurrent works on the saved one too');

console.log(process.exitCode ? '\nFAILURES' : '\nall green');
