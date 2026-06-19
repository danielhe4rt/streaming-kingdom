/*
 * streams-toolkit — voice-roster bridge, injected into Vesktop via CDP.
 *
 * Runs in the Discord renderer (no Vencord plugin / build needed): reads the
 * already-loaded Vencord webpack stores, subscribes to Flux voice + speaking
 * events, and pushes camelCase roster snapshots to the toolkit's bridge WS
 * ingress. `__PORT__` is replaced with the configured bridge port before eval.
 * Idempotent: re-evaluating returns "already-injected" without re-subscribing.
 */
(() => {
  if (window.__stkVoiceBridge) return "already-injected";
  window.__stkVoiceBridge = true;

  const W = window.Vencord.Webpack;
  const C = W.Common || {};
  const findStore = W.findStore || (n => W.findByStoreName && W.findByStoreName(n));
  const SelectedChannelStore = C.SelectedChannelStore || findStore("SelectedChannelStore");
  const VoiceStateStore = findStore("VoiceStateStore");
  const ChannelStore = C.ChannelStore || findStore("ChannelStore");
  const UserStore = C.UserStore || findStore("UserStore");
  const GuildMemberStore = C.GuildMemberStore || findStore("GuildMemberStore");
  const FluxDispatcher = C.FluxDispatcher;

  const speaking = new Set();
  let ws = null, retry = null, last = null;

  const avatarUrl = (id, hash) =>
    hash ? "https://cdn.discordapp.com/avatars/" + id + "/" + hash +
      (hash.startsWith("a_") ? ".gif" : ".png") : null;

  function snapshot() {
    const channelId = SelectedChannelStore.getVoiceChannelId();
    if (!channelId) return { channelId: null, channelName: null, members: [] };
    const channel = ChannelStore.getChannel(channelId);
    const guildId = channel && channel.guild_id;
    const states = VoiceStateStore.getVoiceStatesForChannel(channelId) || {};
    const members = Object.values(states).map(s => {
      const u = UserStore.getUser(s.userId) || {};
      const nick = guildId && GuildMemberStore.getNick ? GuildMemberStore.getNick(guildId, s.userId) : null;
      return {
        userId: s.userId,
        displayName: nick || u.globalName || u.username || s.userId,
        avatarUrl: avatarUrl(s.userId, u.avatar),
        speaking: speaking.has(s.userId),
        selfMute: !!s.selfMute, selfDeaf: !!s.selfDeaf,
        serverMute: !!s.mute, serverDeaf: !!s.deaf,
      };
    });
    return { channelId, channelName: (channel && channel.name) || null, members };
  }

  function push() {
    last = JSON.stringify(snapshot());
    if (ws && ws.readyState === 1) ws.send(last);
  }

  function schedule() {
    ws = null;
    if (retry) return;
    retry = setTimeout(() => { retry = null; connect(); }, 3000);
  }

  function connect() {
    try { ws = new WebSocket("ws://127.0.0.1:__PORT__/"); }
    catch (e) { return schedule(); }
    ws.onopen = () => { last ? ws.send(last) : push(); };
    ws.onclose = schedule;
    ws.onerror = () => { try { ws.close(); } catch (e) {} };
  }

  FluxDispatcher.subscribe("SPEAKING", e => {
    if (e.context && e.context !== "default") return;
    if (e.speakingFlags) speaking.add(e.userId); else speaking.delete(e.userId);
    push();
  });
  FluxDispatcher.subscribe("VOICE_STATE_UPDATES", push);
  FluxDispatcher.subscribe("VOICE_CHANNEL_SELECT", () => { speaking.clear(); push(); });

  connect();
  return "injected";
})()
