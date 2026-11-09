/**
 * MORI 商城前端。原生 ES Module + JSDoc 类型，无构建步骤。
 *
 * @typedef {Object} User
 * @property {number} id
 * @property {string} username
 * @property {string} displayName
 * @property {"admin"|"seller"} role
 * @property {string} status
 *
 * @typedef {Object} Product
 * @property {number} id
 * @property {string} ownerName
 * @property {string} name
 * @property {string} subtitle
 * @property {string} description
 * @property {string} category
 * @property {number} price
 * @property {string} imageUrl
 * @property {string} status
 * @property {number} stock
 * @property {string} createdAt
 *
 * @typedef {Object} Account
 * @property {number} id
 * @property {string} username
 * @property {string} displayName
 * @property {"admin"|"seller"} role
 * @property {string} status
 * @property {?string} rejectionReason
 * @property {?string} approvedAt
 * @property {?string} lastLoginAt
 * @property {number} loginCount
 * @property {number} activeProductCount
 * @property {string} createdAt
 *
 * @typedef {Object} Dashboard
 * @property {{totalAccounts:number,pendingAccounts:number,onlineAccounts:number,todayLogins:number,activeProducts:number}} stats
 * @property {Account[]} accounts
 * @property {{date:string,count:number}[]} loginSeries
 * @property {{id:number,username:string,success:boolean,reason:?string,createdAt:string}[]} recentLogins
 */

const $ = (selector) => document.querySelector(selector);

/**
 * @param {string} tag
 * @param {Record<string, any>} [props]
 * @param {...any} children
 */
function el(tag, props = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value === null || value === undefined || value === false) continue;
    if (key === "class") node.className = value;
    else if (key === "dataset") Object.assign(node.dataset, value);
    else if (key.startsWith("on")) node.addEventListener(key.slice(2).toLowerCase(), value);
    else node.setAttribute(key, String(value));
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child instanceof Node ? child : String(child));
  }
  return node;
}

const SVG_NS = "http://www.w3.org/2000/svg";
/** @param {string} tag @param {Record<string, any>} attrs @param {...any} children */
function svg(tag, attrs = {}, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, String(value));
  for (const child of children.flat()) if (child) node.append(child);
  return node;
}

/**
 * @param {string} path
 * @param {{method?:string, body?:any}} [options]
 * @returns {Promise<any>}
 */
async function api(path, options = {}) {
  const response = await fetch(`/api${path}`, {
    method: options.method || "GET",
    credentials: "same-origin",
    headers: options.body ? { "Content-Type": "application/json" } : undefined,
    body: options.body ? JSON.stringify(options.body) : undefined,
  });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    const error = new Error(data.message || `请求失败（${response.status}）`);
    // @ts-ignore
    error.status = response.status;
    throw error;
  }
  return data;
}

const STATUS_TEXT = {
  pending: "待审核",
  approved: "已通过",
  rejected: "已驳回",
  disabled: "已停用",
  draft: "草稿",
  active: "在售",
  offline: "已下架",
};

const STATUS_TONE = {
  pending: "is-warn",
  rejected: "is-bad",
  disabled: "is-bad",
  approved: "is-ok",
  active: "is-ok",
};

/** @param {string} status */
const statusTag = (status) =>
  el("span", { class: `tag ${STATUS_TONE[status] || ""}` }, STATUS_TEXT[status] || status);

/** @param {?string} value */
const formatTime = (value) => (value ? value.slice(0, 16).replace("T", " ") : "—");

let toastTimer = 0;
/** @param {string} message */
function toast(message) {
  const node = $("#toast");
  node.textContent = message;
  node.classList.add("is-visible");
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => node.classList.remove("is-visible"), 2400);
}

/** @type {{user: ?User, category: string, keyword: string}} */
const state = { user: null, category: "", keyword: "" };

/* ---------------- 商城 ---------------- */

/** @param {Product[]} products */
function renderProducts(products) {
  const grid = $("#product-grid");
  grid.replaceChildren(
    ...products.map((product) =>
      el(
        "article",
        { class: "product-card" },
        el("img", {
          class: "product-thumb",
          src: product.imageUrl,
          alt: product.name,
          loading: "lazy",
        }),
        el(
          "div",
          { class: "product-body" },
          el("p", { class: "product-category" }, product.category),
          el("h3", { class: "product-name" }, product.name),
          el("p", { class: "product-subtitle" }, product.subtitle || product.ownerName),
          el(
            "div",
            { class: "product-foot" },
            el("strong", { class: "product-price" }, `¥${product.price.toFixed(2)}`),
            el("span", { class: "product-stock" }, `库存 ${product.stock}`),
          ),
        ),
      ),
    ),
  );
  $("#product-count").textContent = products.length ? `共 ${products.length} 件商品` : "";
  $("#product-empty").hidden = products.length > 0;
}

/** @param {string[]} categories */
function renderCategories(categories) {
  const list = $("#category-list");
  const items = [{ value: "", label: "全部商品" }].concat(
    categories.map((category) => ({ value: category, label: category })),
  );
  list.replaceChildren(
    ...items.map((item) =>
      el(
        "button",
        {
          type: "button",
          class: `category-item${state.category === item.value ? " is-active" : ""}`,
          onclick: () => {
            state.category = item.value;
            state.keyword = "";
            /** @type {HTMLInputElement} */ ($("#search-input")).value = "";
            loadStorefront();
          },
        },
        el("span", {}, item.label),
      ),
    ),
  );
}

async function loadStorefront() {
  const params = new URLSearchParams();
  if (state.category) params.set("category", state.category);
  if (state.keyword) params.set("q", state.keyword);
  try {
    const data = await api(`/storefront?${params}`);
    $("#product-heading").textContent = state.category || "全部商品";
    renderCategories(data.categories);
    renderProducts(data.products);
  } catch (error) {
    toast(/** @type {Error} */ (error).message);
  }
}

/* ---------------- 账号 ---------------- */

function updateAuthUI() {
  const entry = $("#account-entry");
  const topButton = $("#top-auth-button");
  const user = state.user;

  entry.querySelector(".account-avatar").textContent = user ? user.displayName.slice(0, 1) : "访";
  entry.querySelector("strong").textContent = user ? user.displayName : "登录 / 注册";
  entry.querySelector("small").textContent = user
    ? user.role === "admin"
      ? "管理员 · 进入后台"
      : "商家 · 进入工作台"
    : "管理你的商店";

  topButton.textContent = user ? "退出登录" : "登录";
}

/** @param {boolean} open */
function toggleAuth(open) {
  const dialog = /** @type {HTMLDialogElement} */ ($("#auth-dialog"));
  if (open) dialog.showModal();
  else dialog.close();
}

/** @param {"login"|"register"} mode */
function switchAuthTab(mode) {
  for (const button of document.querySelectorAll("[data-auth-mode]")) {
    const target = /** @type {HTMLElement} */ (button);
    const active = target.dataset.authMode === mode;
    target.classList.toggle("is-active", active);
    target.setAttribute("aria-selected", String(active));
  }
  $("#login-form").hidden = mode !== "login";
  $("#register-form").hidden = mode === "login";
}

/** @param {string} selector @param {string} message @param {boolean} [ok] */
function setMessage(selector, message, ok = false) {
  const node = $(selector);
  node.textContent = message;
  node.classList.toggle("is-ok", ok);
}

async function refreshUser() {
  try {
    const data = await api("/auth/me");
    state.user = data.user;
  } catch {
    state.user = null;
  }
  updateAuthUI();
}

/* ---------------- 后台 ---------------- */

function showStore() {
  $("#store-view").hidden = false;
  $("#portal-view").hidden = true;
  loadStorefront();
}

/** @param {string} hash */
async function showPortal(hash) {
  const user = state.user;
  if (!user) {
    location.hash = "#/store";
    toggleAuth(true);
    return;
  }

  const isAdmin = user.role === "admin";
  const routes = isAdmin
    ? [
        { hash: "#/portal/dashboard", label: "数据看板", render: renderDashboard },
        { hash: "#/portal/accounts", label: "账号管理", render: renderAccounts },
      ]
    : [{ hash: "#/portal/products", label: "商品管理", render: renderProductsPage }];

  const current = routes.find((item) => item.hash === hash) || routes[0];
  if (current.hash !== hash) location.hash = current.hash;

  $("#store-view").hidden = true;
  $("#portal-view").hidden = false;
  $("#portal-avatar").textContent = user.displayName.slice(0, 1);
  $("#portal-user-name").textContent = user.displayName;
  $("#portal-user-role").textContent = isAdmin ? "管理员" : "商家";

  $("#portal-nav").replaceChildren(
    ...routes.map((item) =>
      el(
        "button",
        {
          type: "button",
          class: item === current ? "is-active" : "",
          onclick: () => {
            location.hash = item.hash;
          },
        },
        item.label,
      ),
    ),
    el(
      "button",
      {
        type: "button",
        onclick: async () => {
          await api("/auth/logout", { method: "POST" }).catch(() => {});
          state.user = null;
          updateAuthUI();
          location.hash = "#/store";
          toast("已退出登录");
        },
      },
      "退出登录",
    ),
  );

  await current.render();
}

/** @param {string} title @param {string} [description] */
function pageHead(title, description = "") {
  return el(
    "header",
    { class: "page-head" },
    el("div", {}, el("h1", {}, title), description ? el("p", {}, description) : ""),
  );
}

/* ---- 商家：商品管理 ---- */

async function renderProductsPage() {
  const main = $("#portal-main");
  main.replaceChildren(pageHead("商品管理", "创建商品并控制上架状态，只有「在售」会出现在商城。"));

  const formId = "create-product-form";
  const form = el(
    "form",
    { class: "panel", id: formId },
    el("div", { class: "panel-head" }, el("h2", {}, "新建商品")),
    el(
      "div",
      { class: "form-grid" },
      el("label", { class: "field" }, "商品名称", el("input", { name: "name", required: true, minlength: 2, maxlength: 120 })),
      el(
        "label",
        { class: "field" },
        "分类",
        el("input", { name: "category", required: true, maxlength: 40, list: "category-options" }),
        el(
          "datalist",
          { id: "category-options" },
          ...["数码", "家居", "穿搭", "食品", "户外"].map((item) => el("option", { value: item })),
        ),
      ),
      el("label", { class: "field" }, "价格（元）", el("input", { name: "price", type: "number", step: "0.01", min: "0.01", required: true })),
      el("label", { class: "field" }, "库存", el("input", { name: "stock", type: "number", min: "0", step: "1", value: "0", required: true })),
      el("label", { class: "field" }, "副标题", el("input", { name: "subtitle", maxlength: 180 })),
      el(
        "label",
        { class: "field" },
        "状态",
        el("select", { name: "status" }, el("option", { value: "draft" }, "草稿"), el("option", { value: "active" }, "在售")),
      ),
      el("label", { class: "field is-wide" }, "图片地址", el("input", { name: "imageUrl", maxlength: 500, placeholder: "留空则按分类使用默认图" })),
      el("label", { class: "field is-wide" }, "商品描述", el("textarea", { name: "description", required: true, maxlength: 5000 })),
    ),
    el(
      "div",
      { class: "form-actions" },
      el("button", { class: "primary-button", type: "submit" }, "保存商品"),
      el("span", { class: "form-message", id: "product-message" }),
    ),
  );

  const listPanel = el("section", { class: "panel" }, el("div", { class: "panel-head" }, el("h2", {}, "我的商品")), el("p", {}, "载入中…"));
  main.append(form, listPanel);

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const data = Object.fromEntries(new FormData(/** @type {HTMLFormElement} */ (form)));
    try {
      await api("/seller/products", {
        method: "POST",
        body: {
          name: data.name,
          subtitle: data.subtitle,
          description: data.description,
          category: data.category,
          price: Number(data.price),
          stock: Number(data.stock),
          status: data.status,
          imageUrl: data.imageUrl,
        },
      });
      /** @type {HTMLFormElement} */ (form).reset();
      setMessage("#product-message", "商品已创建", true);
      toast("商品已创建");
      await loadOwnProducts(listPanel);
    } catch (error) {
      setMessage("#product-message", /** @type {Error} */ (error).message);
    }
  });

  await loadOwnProducts(listPanel);
}

/** @param {HTMLElement} panel */
async function loadOwnProducts(panel) {
  let products = [];
  try {
    products = await api("/seller/products");
  } catch (error) {
    panel.replaceChildren(el("p", {}, /** @type {Error} */ (error).message));
    return;
  }

  const body = el(
    "tbody",
    {},
    ...products.map((product) =>
      el(
        "tr",
        {},
        el("td", {}, product.name),
        el("td", {}, product.category),
        el("td", { class: "num" }, `¥${product.price.toFixed(2)}`),
        el("td", { class: "num" }, String(product.stock)),
        el("td", {}, statusTag(product.status)),
        el(
          "td",
          {},
          el(
            "div",
            { class: "row-actions" },
            ...["active", "draft", "offline"]
              .filter((status) => status !== product.status)
              .map((status) =>
                el(
                  "button",
                  {
                    type: "button",
                    class: status === "offline" ? "is-danger" : "",
                    onclick: async () => {
                      try {
                        await api(`/seller/products/${product.id}/status`, {
                          method: "PATCH",
                          body: { status },
                        });
                        await loadOwnProducts(panel);
                        toast(`已设为${STATUS_TEXT[status]}`);
                      } catch (error) {
                        toast(/** @type {Error} */ (error).message);
                      }
                    },
                  },
                  status === "active" ? "上架" : status === "draft" ? "转草稿" : "下架",
                ),
              ),
          ),
        ),
      ),
    ),
  );

  panel.replaceChildren(
    el(
      "div",
      { class: "panel-head" },
      el("h2", {}, "我的商品"),
      el("small", {}, `共 ${products.length} 件`),
    ),
    products.length
      ? el(
          "div",
          { class: "table-scroll" },
          el(
            "table",
            { class: "data-table" },
            el(
              "thead",
              {},
              el(
                "tr",
                {},
                ...["商品", "分类", "价格", "库存", "状态", "操作"].map((label, index) =>
                  el("th", { class: index === 2 || index === 3 ? "num" : "" }, label),
                ),
              ),
            ),
            body,
          ),
        )
      : el("p", {}, "还没有商品，先在上方创建一件。"),
  );
}

/* ---- 管理员：数据看板 ---- */

async function renderDashboard() {
  const main = $("#portal-main");
  main.replaceChildren(pageHead("数据看板", "账号登录情况与商品上架概览。"), el("p", {}, "载入中…"));

  let data;
  try {
    data = await api("/admin/dashboard");
  } catch (error) {
    main.replaceChildren(pageHead("数据看板"), el("p", {}, /** @type {Error} */ (error).message));
    return;
  }

  const stats = [
    { label: "账号总数", value: data.stats.totalAccounts },
    { label: "待审核", value: data.stats.pendingAccounts, accent: data.stats.pendingAccounts > 0 },
    { label: "在线账号", value: data.stats.onlineAccounts },
    { label: "今日登录", value: data.stats.todayLogins },
    { label: "在售商品", value: data.stats.activeProducts },
  ];

  const statColumn = el(
    "aside",
    { class: "stat-column" },
    ...stats.map((item) =>
      el(
        "div",
        { class: `stat-card${item.accent ? " is-accent" : ""}` },
        el("span", {}, item.label),
        el("strong", {}, String(item.value)),
      ),
    ),
  );

  const right = el("div", {});
  right.append(
    el(
      "section",
      { class: "panel" },
      el("div", { class: "panel-head" }, el("h2", {}, "近 7 日登录"), el("small", {}, "成功登录次数")),
      renderChart(data.loginSeries),
    ),
    el(
      "section",
      { class: "panel" },
      el("div", { class: "panel-head" }, el("h2", {}, "账号与上架产品"), el("small", {}, `共 ${data.accounts.length} 个账号`)),
      accountsTable(data.accounts, true),
    ),
    el(
      "section",
      { class: "panel" },
      el("div", { class: "panel-head" }, el("h2", {}, "最近登录记录"), el("small", {}, "最新 20 条")),
      data.recentLogins.length
        ? el(
            "div",
            { class: "table-scroll" },
            el(
              "table",
              { class: "data-table" },
              el(
                "thead",
                {},
                el("tr", {}, ...["账号", "结果", "原因", "时间"].map((label) => el("th", {}, label))),
              ),
              el(
                "tbody",
                {},
                ...data.recentLogins.map((item) =>
                  el(
                    "tr",
                    {},
                    el("td", {}, item.username),
                    el(
                      "td",
                      {},
                      el(
                        "span",
                        { class: `tag ${item.success ? "is-ok" : "is-bad"}` },
                        item.success ? "成功" : "失败",
                      ),
                    ),
                    el("td", {}, item.reason || "—"),
                    el("td", {}, formatTime(item.createdAt)),
                  ),
                ),
              ),
            ),
          )
        : el("p", {}, "暂无登录记录。"),
    ),
  );

  main.replaceChildren(
    pageHead("数据看板", "账号登录情况与商品上架概览。"),
    el("div", { class: "dashboard-grid" }, statColumn, right),
  );
}

/** @param {{date:string,count:number}[]} series */
function renderChart(series) {
  const width = 640;
  const height = 220;
  const padL = 30;
  const padR = 12;
  const padT = 14;
  const padB = 26;
  const innerW = width - padL - padR;
  const innerH = height - padT - padB;
  const max = Math.max(1, ...series.map((point) => point.count));
  const x = (index) => padL + (series.length < 2 ? innerW / 2 : (index * innerW) / (series.length - 1));
  const y = (value) => padT + innerH - (value / max) * innerH;

  const points = series.map((point, index) => [x(index), y(point.count)]);
  const line = points.map(([px, py], index) => `${index ? "L" : "M"}${px.toFixed(1)} ${py.toFixed(1)}`).join(" ");
  const base = padT + innerH;
  const area = `${line} L${x(series.length - 1).toFixed(1)} ${base} L${padL} ${base} Z`;

  const grid = [0, 0.5, 1].flatMap((ratio) => {
    const gy = padT + innerH * (1 - ratio);
    return [
      svg("line", { class: "chart-grid", x1: padL, x2: width - padR, y1: gy, y2: gy }),
      svg("text", { class: "chart-label", x: 0, y: gy + 3 }, String(Math.round(max * ratio))),
    ];
  });

  return svg(
    "svg",
    { class: "chart", viewBox: `0 0 ${width} ${height}`, role: "img", "aria-label": "近 7 日登录趋势" },
    ...grid,
    svg("path", { class: "chart-area", d: area }),
    svg("path", { class: "chart-line", d: line }),
    ...points.map(([px, py]) => svg("circle", { class: "chart-dot", cx: px, cy: py, r: 3 })),
    ...series.map((point, index) =>
      svg(
        "text",
        { class: "chart-label", x: x(index), y: height - 6, "text-anchor": "middle" },
        point.date.slice(5),
      ),
    ),
  );
}

/** @param {Account[]} accounts @param {boolean} compact */
function accountsTable(accounts, compact = false) {
  const head = compact
    ? ["账号", "状态", "登录次数", "上架产品", "最后登录"]
    : ["账号", "角色", "状态", "登录次数", "上架产品", "最后登录", "操作"];

  const row = (account) => {
    const cells = [
      el("td", {}, el("div", {}, account.displayName || account.username), el("small", {}, `@${account.username}`)),
    ];
    if (!compact) cells.push(el("td", {}, account.role === "admin" ? "管理员" : "商家"));
    cells.push(el("td", {}, statusTag(account.status)));
    cells.push(el("td", { class: "num" }, String(account.loginCount)));
    cells.push(el("td", { class: "num" }, String(account.activeProductCount)));
    cells.push(el("td", {}, formatTime(account.lastLoginAt)));
    if (!compact) cells.push(el("td", {}, account.role === "admin" ? "—" : accountActions(account)));
    return el("tr", {}, ...cells);
  };

  function accountActions(account) {
    /** @param {string} status @param {string} label @param {boolean} [danger] */
    const button = (status, label, danger = false) =>
      el(
        "button",
        {
          type: "button",
          class: danger ? "is-danger" : "",
          onclick: async () => {
            let reason = "";
            if (status !== "approved") {
              reason = window.prompt(
                `请填写${status === "rejected" ? "驳回" : "停用"}原因（可留空）`,
                status === "rejected" ? "人工审核未通过" : "管理员停用账号",
              ) || "";
            }
            try {
              await api(`/admin/accounts/${account.id}/status`, {
                method: "PATCH",
                body: { status, reason },
              });
              toast(`已将 @${account.username} 设为${STATUS_TEXT[status]}`);
              await renderAccounts();
            } catch (error) {
              toast(/** @type {Error} */ (error).message);
            }
          },
        },
        label,
      );

    return el(
      "div",
      { class: "row-actions" },
      account.status !== "approved" ? button("approved", "通过") : "",
      account.status !== "rejected" ? button("rejected", "驳回", true) : "",
      account.status !== "disabled" ? button("disabled", "停用", true) : "",
    );
  }

  if (!accounts.length) return el("p", {}, "暂无账号。");

  return el(
    "div",
    { class: "table-scroll" },
    el(
      "table",
      { class: "data-table" },
      el("thead", {}, el("tr", {}, ...head.map((label) => el("th", {}, label)))),
      el("tbody", {}, ...accounts.map(row)),
    ),
  );
}

/* ---- 管理员：账号管理 ---- */

/** @param {boolean} [renderHead] */
async function renderAccounts(renderHead = true) {
  const main = $("#portal-main");
  const head = pageHead("账号管理", "新注册账号为待审核状态，通过后才能登录。");

  let accounts;
  try {
    accounts = await api("/admin/accounts");
  } catch (error) {
    main.replaceChildren(head, el("p", {}, /** @type {Error} */ (error).message));
    return;
  }

  const pending = accounts.filter((account) => account.status === "pending").length;
  const panel = el(
    "section",
    { class: "panel" },
    el("div", { class: "panel-head" }, el("h2", {}, "全部账号"), el("small", {}, `待审核 ${pending} 个`)),
    accountsTable(accounts),
  );

  main.replaceChildren(renderHead ? head : el("span"), panel);
}

/* ---------------- 启动 ---------------- */

function bindEvents() {
  $("#search-form").addEventListener("submit", (event) => {
    event.preventDefault();
    state.keyword = /** @type {HTMLInputElement} */ ($("#search-input")).value.trim();
    if (state.keyword) state.category = "";
    location.hash = "#/store";
    loadStorefront();
  });

  $("#account-entry").addEventListener("click", () => {
    if (state.user) location.hash = "#/portal";
    else toggleAuth(true);
  });

  $("#top-auth-button").addEventListener("click", async () => {
    if (!state.user) {
      toggleAuth(true);
      return;
    }
    await api("/auth/logout", { method: "POST" }).catch(() => {});
    state.user = null;
    updateAuthUI();
    location.hash = "#/store";
    toast("已退出登录");
  });

  $("#auth-close").addEventListener("click", () => toggleAuth(false));

  for (const tab of document.querySelectorAll("[data-auth-mode]")) {
    tab.addEventListener("click", () =>
      switchAuthTab(/** @type {HTMLElement} */ (tab).dataset.authMode === "register" ? "register" : "login"),
    );
  }

  $("#login-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const data = Object.fromEntries(new FormData(/** @type {HTMLFormElement} */ (event.target)));
    try {
      const result = await api("/auth/login", { method: "POST", body: data });
      state.user = result.user;
      updateAuthUI();
      toggleAuth(false);
      toast(`欢迎回来，${state.user.displayName}`);
      location.hash = "#/portal";
    } catch (error) {
      setMessage("#login-message", /** @type {Error} */ (error).message);
    }
  });

  $("#register-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const data = Object.fromEntries(new FormData(/** @type {HTMLFormElement} */ (event.target)));
    try {
      const result = await api("/auth/register", { method: "POST", body: data });
      setMessage("#register-message", result.message, true);
      /** @type {HTMLFormElement} */ (event.target).reset();
    } catch (error) {
      setMessage("#register-message", /** @type {Error} */ (error).message);
    }
  });

  document.querySelector("[data-scroll-products]")?.addEventListener("click", () => {
    $("#product-section")?.scrollIntoView({ behavior: "smooth", block: "start" });
  });

  window.addEventListener("hashchange", route);
}

async function route() {
  const hash = location.hash || "#/store";
  if (hash.startsWith("#/portal")) {
    await showPortal(hash);
  } else {
    showStore();
  }
}

async function init() {
  bindEvents();
  await refreshUser();
  await route();
}

init();
