<script setup lang="ts">
import { inject, ref, watch } from 'vue';
import { Cloud, RefreshCw, UploadCloud, DownloadCloud, Eye, EyeOff } from 'lucide-vue-next';
import { api } from '../lib/api';
import { managerKey } from '../lib/context';
import AccentButton from './ui/AccentButton.vue';
import Segmented from './ui/Segmented.vue';

const ctx = inject(managerKey)!;

const provider = ref<'webdav' | 'gist'>('webdav');

// WebDAV 表单
const url = ref('');
const username = ref('');
const password = ref('');
const davEnabled = ref(false);
const davAutoSync = ref(false);

// GitHub Gist 表单
const token = ref('');
const gistId = ref('');
const gistEnabled = ref(false);
const gistAutoSync = ref(false);

const showWebdavPwd = ref(false);
const showToken = ref(false);
const testing = ref(false);
const syncing = ref(false);
const status = ref('');
const statusOk = ref(true);
const formDirty = ref(false);
const syncClipboard = ref(false);

const providerOptions = [
  { id: 'webdav', label: 'WebDAV' },
  { id: 'gist', label: 'GitHub Gist' },
];

function markDirty() {
  formDirty.value = true;
}

function fillFrom(s: import('../types').Settings) {
  provider.value = s.syncProvider === 'gist' ? 'gist' : 'webdav';
  url.value = s.webdav.url;
  username.value = s.webdav.username;
  password.value = s.webdav.password;
  davEnabled.value = s.webdav.enabled;
  davAutoSync.value = s.webdav.autoSync;
  token.value = s.gist.token;
  gistId.value = s.gist.gistId;
  gistEnabled.value = s.gist.enabled;
  gistAutoSync.value = s.gist.autoSync;
  syncClipboard.value = s.syncClipboard ?? false;
  formDirty.value = false;
}

watch(
  ctx.data,
  (d) => {
    if (d && !formDirty.value) fillFrom(d.settings);
  },
  { immediate: true },
);

function switchProvider(p: string) {
  provider.value = p as 'webdav' | 'gist';
  formDirty.value = true;
}

async function save(silent = false): Promise<boolean> {
  const s = ctx.data.value?.settings;
  if (!s) return false;
  try {
    await api.saveSettings({
      ...s,
      syncProvider: provider.value,
      syncClipboard: syncClipboard.value,
      webdav: {
        enabled: davEnabled.value,
        autoSync: davAutoSync.value,
        url: url.value.trim(),
        username: username.value.trim(),
        password: password.value,
      },
      gist: {
        enabled: gistEnabled.value,
        autoSync: gistAutoSync.value,
        token: token.value.trim(),
        gistId: gistId.value.trim(),
      },
    });
    formDirty.value = false;
    await ctx.refresh();
    if (!silent) ctx.toast('同步配置已保存');
    return true;
  } catch (e) {
    ctx.toast(String(e), 'err');
    return false;
  }
}

async function test() {
  testing.value = true;
  status.value = '';
  try {
    const msg =
      provider.value === 'gist'
        ? await api.gistTest(token.value.trim(), gistId.value.trim())
        : await api.webdavTest(url.value.trim(), username.value.trim(), password.value);
    statusOk.value = true;
    status.value = `✓ ${msg}`;
  } catch (e) {
    statusOk.value = false;
    status.value = `✗ ${e}`;
  } finally {
    testing.value = false;
  }
}

async function doSync(direction: 'merge' | 'push' | 'pull') {
  // push/pull 是整体覆盖的破坏性操作，必须先确认
  if (direction === 'push') {
    const ok = await ctx.confirm({
      title: '仅上传：用本机数据覆盖云端？',
      message: '云端现有的提示词与剪贴板历史将被本机数据整体替换，云端被覆盖的内容无法找回。',
      confirmText: '覆盖云端',
      danger: true,
    });
    if (!ok) return;
  } else if (direction === 'pull') {
    const ok = await ctx.confirm({
      title: '仅下载：用云端数据覆盖本机？',
      message:
        '本机现有的提示词与文本剪贴板历史将被云端数据整体替换（本机图片剪贴板会保留），覆盖前的本机内容无法找回。',
      confirmText: '覆盖本机',
      danger: true,
    });
    if (!ok) return;
  }
  syncing.value = true;
  status.value = '';
  try {
    if (!(await save(true))) return;
    const report = await api.syncNow(direction);
    statusOk.value = true;
    status.value = `✓ ${report.message}`;
    await ctx.refresh();
  } catch (e) {
    statusOk.value = false;
    status.value = `✗ ${e}`;
  } finally {
    syncing.value = false;
  }
}

const httpWarn = () => /^http:\/\//i.test(url.value.trim());

const providerName = () => (provider.value === 'gist' ? 'GitHub Gist' : 'WebDAV');
const providerOn = () => (provider.value === 'gist' ? gistEnabled.value : davEnabled.value);
</script>

<template>
  <div class="sync">
    <header class="sync-head">
      <h2 class="sync-title">云同步</h2>
      <span class="on-badge" :class="{ on: providerOn() }">
        <span class="on-dot" />
        {{ providerOn() ? '已启用' : '未启用' }}
      </span>
      <span class="grow" />
    </header>

    <div class="sync-body">
      <!-- 状态总览卡 -->
      <div class="overview card">
        <div class="ov-icon">
          <Cloud :size="21" :stroke-width="1.8" />
        </div>
        <div class="ov-info">
          <div class="ov-title">{{ providerName() }}</div>
          <div class="ov-desc muted">条目级合并：同条目保留较新版本，删除跨设备传播</div>
        </div>
        <div class="ov-actions">
          <button class="ob" :disabled="syncing" title="双向合并" @click="doSync('merge')">
            <RefreshCw :size="14" :class="{ spin: syncing }" /> 立即同步
          </button>
          <button class="ob" :disabled="syncing" title="本机覆盖云端" @click="doSync('push')">
            <UploadCloud :size="14" /> 仅上传
          </button>
          <button class="ob" :disabled="syncing" title="云端覆盖本机" @click="doSync('pull')">
            <DownloadCloud :size="14" /> 仅下载
          </button>
        </div>
      </div>

      <!-- 后端切换 -->
      <Segmented :model-value="provider" :options="providerOptions" @update:model-value="switchProvider" />

      <!-- 同步范围（与后端无关的公共选项） -->
      <div class="form card">
        <label class="row opt">
          <span class="switch">
            <input v-model="syncClipboard" type="checkbox" @change="markDirty" />
            <span class="track"><span class="thumb" /></span>
          </span>
          <span>
            云同步包含剪贴板历史
            <small class="muted block-note">
              默认关闭。剪贴板里常出现密码等敏感内容，开启后文本历史会随同步上传到你的网盘 / Gist
            </small>
            <small v-if="syncClipboard" class="muted block-note">
              注意：多设备间此开关需保持一致，否则云端剪贴板内容会随各端设置互相覆盖
            </small>
          </span>
        </label>
        <p v-if="provider === 'webdav' && httpWarn()" class="muted hint warn-hint">
          当前使用 http:// 连接，账号密码将以明文传输；如服务器支持，建议改用 https://
        </p>
      </div>

      <!-- WebDAV 表单 -->
      <div v-if="provider === 'webdav'" class="form card">
        <p class="muted hint">
          推荐坚果云：<span class="mono">https://dav.jianguoyun.com/dav/promptmate/</span>
          （末级目录自动创建；密码使用应用密码）
        </p>
        <label class="field">
          <span>服务器地址</span>
          <input
            v-model="url"
            type="text"
            placeholder="https://dav.jianguoyun.com/dav/promptmate/"
            spellcheck="false"
            @input="markDirty"
          />
        </label>
        <div class="row-fields">
          <label class="field grow">
            <span>账号</span>
            <input v-model="username" type="text" autocomplete="off" spellcheck="false" @input="markDirty" />
          </label>
          <label class="field grow">
            <span>密码 / 应用密码</span>
            <span class="pwd-box">
              <input
                v-model="password"
                :type="showWebdavPwd ? 'text' : 'password'"
                autocomplete="new-password"
                @input="markDirty"
              />
              <button
                class="eye"
                type="button"
                :aria-label="showWebdavPwd ? '隐藏密码' : '显示密码'"
                @click="showWebdavPwd = !showWebdavPwd"
              >
                <Eye v-if="showWebdavPwd" :size="14" />
                <EyeOff v-else :size="14" />
              </button>
            </span>
          </label>
        </div>
        <label class="row opt">
          <span class="switch">
            <input v-model="davEnabled" type="checkbox" @change="markDirty" />
            <span class="track"><span class="thumb" /></span>
          </span>
          <span>启用 WebDAV 同步</span>
        </label>
        <label class="row opt">
          <span class="switch">
            <input v-model="davAutoSync" type="checkbox" @change="markDirty" />
            <span class="track"><span class="thumb" /></span>
          </span>
          <span>自动同步（启动时与内容变更后自动合并）</span>
        </label>
      </div>

      <!-- GitHub Gist 表单 -->
      <div v-else class="form card">
        <p class="muted hint">
          数据保存在你的 <b>secret Gist</b>（不公开、仅凭 Token 可访问、自带版本历史）。
          Token 创建：GitHub → Settings → Developer settings →
          <b>Personal access tokens (classic)</b> → 勾选 <span class="mono">gist</span> 权限。
        </p>
        <label class="field">
          <span>GitHub Token</span>
          <span class="pwd-box">
            <input
              v-model="token"
              :type="showToken ? 'text' : 'password'"
              placeholder="ghp_… / github_pat_…"
              autocomplete="new-password"
              spellcheck="false"
              @input="markDirty"
            />
            <button
              class="eye"
              type="button"
              :aria-label="showToken ? '隐藏 Token' : '显示 Token'"
              @click="showToken = !showToken"
            >
              <Eye v-if="showToken" :size="14" />
              <EyeOff v-else :size="14" />
            </button>
          </span>
        </label>
        <label class="field">
          <span>Gist ID（留空则首次同步时自动创建）</span>
          <input
            v-model="gistId"
            type="text"
            placeholder="自动创建后回填显示"
            spellcheck="false"
            @input="markDirty"
          />
        </label>
        <label class="row opt">
          <span class="switch">
            <input v-model="gistEnabled" type="checkbox" @change="markDirty" />
            <span class="track"><span class="thumb" /></span>
          </span>
          <span>启用 GitHub 同步</span>
        </label>
        <label class="row opt">
          <span class="switch">
            <input v-model="gistAutoSync" type="checkbox" @change="markDirty" />
            <span class="track"><span class="thumb" /></span>
          </span>
          <span>自动同步（启动时与内容变更后自动合并）</span>
        </label>
      </div>

      <!-- 操作行 -->
      <div class="btns">
        <AccentButton :disabled="testing || syncing" @click="save()">保存配置</AccentButton>
        <button :disabled="testing" @click="test">{{ testing ? '测试中…' : '测试连接' }}</button>
      </div>

      <!-- 终端式状态条 -->
      <div v-if="status" class="status tnum" :class="{ ok: statusOk, err: !statusOk }">
        <span class="status-prompt mono">pm</span>
        {{ status }}
      </div>

      <div class="note faint">
        「立即同步 / 仅上传 / 仅下载」会先自动保存当前表单配置。
        凭据仅保存在本机（data.json），不会随数据上传。
      </div>
    </div>
  </div>
</template>

<style scoped>
.sync {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.sync-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 18px;
  height: 52px;
  flex: none;
  border-bottom: 1px solid var(--border);
}

.sync-title {
  font-size: var(--fs-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.on-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--faint);
  background: var(--panel-2);
  border: 1px solid var(--border);
  padding: 2.5px 10px;
  border-radius: 999px;
}

.on-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--faint);
}

.on-badge.on {
  color: var(--ok);
  background: var(--ok-soft);
  border-color: transparent;
}

.on-badge.on .on-dot {
  background: var(--ok);
  box-shadow: 0 0 6px var(--ok);
}

.sync-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 18px 22px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 700px;
  width: 100%;
}

.overview {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 16px 18px;
  position: relative;
  overflow: hidden;
}

.overview::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--brand-grad);
}

.ov-icon {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: var(--brand-soft);
  color: var(--brand);
  flex: none;
}

.ov-info {
  flex: 1;
  min-width: 0;
}

.ov-title {
  font-weight: 650;
  font-size: var(--fs-lg);
}

.ov-desc {
  font-size: 12px;
  margin-top: 3px;
}

.ov-actions {
  display: flex;
  gap: 7px;
  flex: none;
}

.ob {
  font-size: 12px;
  padding: 7px 13px;
  gap: 6px;
}

.spin {
  animation: spin 900ms linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.form {
  display: flex;
  flex-direction: column;
  gap: 13px;
  padding: 16px 18px;
}

.hint {
  font-size: 12px;
  line-height: 1.7;
  padding: 9px 12px;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
}

.mono {
  font-family: var(--font-mono);
  color: var(--brand);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.pwd-box {
  position: relative;
  display: flex;
  align-items: center;
}

.pwd-box input {
  width: 100%;
  padding-right: 34px;
}

.pwd-box .eye {
  position: absolute;
  right: 4px;
  width: 28px;
  height: 28px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--muted);
}

.field > span {
  font-size: 12px;
  color: var(--muted);
}

.row-fields {
  display: flex;
  gap: 10px;
}

.opt {
  gap: 10px;
  font-size: 12.5px;
  cursor: pointer;
}

.block-note {
  display: block;
  margin-top: 3px;
  line-height: 1.6;
  max-width: 520px;
}

.warn-hint {
  color: var(--warn);
  border-color: var(--warn);
}

.btns {
  display: flex;
  gap: 8px;
  align-items: center;
}

.status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 9px 12px;
  border-radius: var(--r-sm);
  font-size: 12.5px;
  border: 1px solid;
}

.status-prompt {
  font-size: 10.5px;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--panel-3);
  color: var(--muted);
}

.status.ok {
  border-color: var(--ok);
  color: var(--ok);
  background: var(--ok-soft);
}

.status.err {
  border-color: var(--danger-btn);
  color: var(--danger);
  background: var(--danger-soft);
}

.note {
  font-size: 11.5px;
  line-height: 1.7;
  padding-bottom: 8px;
}
</style>
