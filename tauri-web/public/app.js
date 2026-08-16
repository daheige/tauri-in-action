// 内嵌 axum Web 服务地址（与 config/app.yaml 的 app_port 一致）
const API_BASE = "http://127.0.0.1:1338";

const statusEl = document.getElementById("status");
const tbody = document.getElementById("user-body");
const pageInfoEl = document.getElementById("page-info");
const pageSizeEl = document.getElementById("page-size");
const newUsernameEl = document.getElementById("new-username");

let currentPage = 1;

function setStatus(text, kind) {
  statusEl.textContent = text;
  statusEl.className = "status " + (kind || "");
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  }[c]));
}

// ---------------------------------------------------------------------------
// 分页查询（HTTP API）
// ---------------------------------------------------------------------------
async function loadUsers() {
  const pageSize = Number(pageSizeEl.value);
  setStatus("正在加载第 " + currentPage + " 页…");
  try {
    const res = await fetch(`${API_BASE}/api/users?page=${currentPage}&page_size=${pageSize}`);
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    const data = await res.json();
    renderUsers(data.items);
    const totalPages = Math.max(1, Math.ceil(data.total / data.page_size));
    pageInfoEl.textContent = `第 ${data.page} / ${totalPages} 页（共 ${data.total} 条）`;
    document.getElementById("btn-prev").disabled = data.page <= 1;
    document.getElementById("btn-next").disabled = data.page >= totalPages;
    setStatus(`分页查询成功：第 ${data.page} 页，共 ${data.total} 条，当前页 ${data.items.length} 条`);
  } catch (e) {
    setStatus("加载失败: " + e.message, "error");
  }
}

// ---------------------------------------------------------------------------
// 渲染表格（含行内编辑）
// ---------------------------------------------------------------------------
function renderUsers(items) {
  tbody.innerHTML = "";
  if (!items || items.length === 0) {
    tbody.innerHTML = '<tr><td colspan="3" class="empty">暂无数据</td></tr>';
    return;
  }
  for (const u of items) {
    const tr = document.createElement("tr");
    tr.dataset.id = u.id;
    tr.innerHTML = `
      <td>${u.id}</td>
      <td class="username-cell">${escapeHtml(u.username)}</td>
      <td class="actions">
        <button class="ghost btn-edit">编辑</button>
        <button class="ghost danger btn-delete">删除</button>
      </td>`;
    tr.querySelector(".btn-edit").addEventListener("click", () => startEdit(tr));
    tr.querySelector(".btn-delete").addEventListener("click", () => deleteUser(u.id));
    tbody.appendChild(tr);
  }
}

function startEdit(tr) {
  const cell = tr.querySelector(".username-cell");
  const current = cell.textContent;
  const input = document.createElement("input");
  input.type = "text";
  input.value = current;
  input.maxLength = 100;
  cell.innerHTML = "";
  cell.appendChild(input);
  input.focus();

  const actions = tr.querySelector(".actions");
  actions.innerHTML = "";
  const btnSave = document.createElement("button");
  btnSave.className = "primary";
  btnSave.textContent = "保存";
  const btnCancel = document.createElement("button");
  btnCancel.className = "ghost";
  btnCancel.textContent = "取消";
  actions.append(btnSave, btnCancel);

  const exitEdit = () => {
    const id = Number(tr.dataset.id);
    renderUsers([]);
    // 重新加载当前页还原行
    loadUsers();
  };

  btnCancel.addEventListener("click", exitEdit);
  btnSave.addEventListener("click", async () => {
    const username = input.value.trim();
    if (!username) return setStatus("用户名不能为空", "error");
    await updateUser(Number(tr.dataset.id), username);
  });
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") btnSave.click();
    if (e.key === "Escape") btnCancel.click();
  });
}

// ---------------------------------------------------------------------------
// 新增
// ---------------------------------------------------------------------------
async function createUser() {
  const username = newUsernameEl.value.trim();
  if (!username) return setStatus("请输入用户名", "error");
  setStatus("正在新增用户…");
  try {
    const res = await fetch(`${API_BASE}/api/users`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    const user = await res.json();
    newUsernameEl.value = "";
    currentPage = 1; // 回到第一页，新数据排在最前
    await loadUsers();
    setStatus(`新增成功：id=${user.id}, username=${user.username}`, "ok");
  } catch (e) {
    setStatus("新增失败: " + e.message, "error");
  }
}

// ---------------------------------------------------------------------------
// 更新
// ---------------------------------------------------------------------------
async function updateUser(id, username) {
  setStatus("正在更新用户…");
  try {
    const res = await fetch(`${API_BASE}/api/users/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    const user = await res.json();
    await loadUsers();
    setStatus(`更新成功：id=${user.id}, username=${user.username}`, "ok");
  } catch (e) {
    setStatus("更新失败: " + e.message, "error");
  }
}

// ---------------------------------------------------------------------------
// 删除
// ---------------------------------------------------------------------------
async function deleteUser(id) {
  if (!confirm(`确认删除用户 id=${id} 吗？`)) return;
  setStatus("正在删除用户…");
  try {
    const res = await fetch(`${API_BASE}/api/users/${id}`, { method: "DELETE" });
    if (!res.ok) throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    await loadUsers();
    setStatus(`删除成功：id=${id}`, "ok");
  } catch (e) {
    setStatus("删除失败: " + e.message, "error");
  }
}

// ---------------------------------------------------------------------------
// Tauri 命令（全量查询，演示用）
// ---------------------------------------------------------------------------
async function loadViaInvoke() {
  setStatus("正在通过 Tauri 命令查询（全量）…");
  try {
    if (!window.__TAURI__) {
      throw new Error("Tauri 全局 API 未注入，请在 tauri.conf.json 的 app 节开启 withGlobalTauri 后重新编译");
    }
    const users = await window.__TAURI__.core.invoke("get_users");
    renderUsers(users);
    pageInfoEl.textContent = `Tauri 命令返回全量 ${users.length} 条（未分页）`;
    setStatus(`Tauri 命令查询成功，共 ${users.length} 条`);
  } catch (e) {
    setStatus("Tauri 命令查询失败: " + e, "error");
  }
}

// ---------------------------------------------------------------------------
// 健康检查
// ---------------------------------------------------------------------------
async function checkHealth() {
  setStatus("正在检查健康状态…");
  try {
    const res = await fetch(`${API_BASE}/api/health`);
    const data = await res.json();
    setStatus(`健康检查: ${JSON.stringify(data)}`);
  } catch (e) {
    setStatus("健康检查失败: " + e.message, "error");
  }
}

// ---------------------------------------------------------------------------
// 事件绑定
// ---------------------------------------------------------------------------
document.getElementById("btn-create").addEventListener("click", createUser);
newUsernameEl.addEventListener("keydown", (e) => {
  if (e.key === "Enter") createUser();
});
document.getElementById("btn-refresh").addEventListener("click", loadUsers);
document.getElementById("btn-invoke").addEventListener("click", loadViaInvoke);
document.getElementById("btn-health").addEventListener("click", checkHealth);
document.getElementById("btn-prev").addEventListener("click", () => {
  currentPage = Math.max(1, currentPage - 1);
  loadUsers();
});
document.getElementById("btn-next").addEventListener("click", () => {
  currentPage += 1;
  loadUsers();
});
pageSizeEl.addEventListener("change", () => {
  currentPage = 1;
  loadUsers();
});

// 启动时自动加载第一页
loadUsers();
