/*!
 * 用于调试自动化 HTTP 端点的自包含 UI 树收集器。
 *
 * 该 IIFE 由 Rust 侧注入主 webview，并挂载 `window.__iNetDebugAutomation.collect()`。
 * `collect` 同步遍历 DOM，并将得到的快照返回给调用方；Rust 侧从调用 `collect`
 * 的脚本的求值结果中读取该快照。该脚本不依赖任何页面框架，且可以安全地重复注入。
 *
 * 快照是按文档顺序排列的简化树节点数组。只有有意义的节点会被保留：
 * 容器、交互元素和文本节点。交互元素不输出其内部子树，并携带聚合后的可见文本，
 * 因此按钮的文本与其自身的点击区域一同出现。
 * 仅用于布局的包装元素不贡献节点，其子孙节点会提升到最近的保留祖先。
 */
(function () {
  "use strict";

  /** 自身元素与整棵子树均被排除出树的 tag。 */
  var SKIP_TAGS = Object.create(null);
  [
    "HEAD",
    "SCRIPT",
    "STYLE",
    "NOSCRIPT",
    "TEMPLATE",
    "LINK",
    "META",
    "BR",
    "HR",
  ].forEach(function (tag) {
    SKIP_TAGS[tag] = true;
  });

  /** 被镜像到每个节点 `attrs` 对象中的元素属性。 */
  var ATTR_WHITELIST = Object.create(null);
  [
    "data-id",
    "data-nodeid",
    "data-handleid",
    "data-handlepos",
    "placeholder",
    "title",
    "alt",
    "href",
    "type",
  ].forEach(function (name) {
    ATTR_WHITELIST[name] = true;
  });

  /** 用于标识天生具有交互性的元素的 role。 */
  var INTERACTIVE_ROLES = Object.create(null);
  [
    "button",
    "link",
    "textbox",
    "checkbox",
    "radio",
    "slider",
    "combobox",
    "option",
  ].forEach(function (role) {
    INTERACTIVE_ROLES[role] = true;
  });

  /** 其元素接受可编辑文本的 role。 */
  var EDITABLE_ROLES = Object.create(null);
  ["textbox", "combobox", "searchbox", "spinbutton"].forEach(function (role) {
    EDITABLE_ROLES[role] = true;
  });

  /** 作为叶节点的 Widget role：收集器不会深入其中。 */
  var ACTION_LEAF_ROLES = Object.create(null);
  [
    "button",
    "link",
    "textbox",
    "searchbox",
    "checkbox",
    "radio",
    "switch",
    "slider",
    "spinbutton",
    "option",
    "tab",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "gridcell",
    "combobox",
    "scrollbar",
    "progressbar",
  ].forEach(function (role) {
    ACTION_LEAF_ROLES[role] = true;
  });

  /** 复合 Widget role：作为节点输出并继续向下遍历。 */
  var ACTION_COMPOSITE_ROLES = Object.create(null);
  [
    "listbox",
    "menu",
    "menubar",
    "radiogroup",
    "tablist",
    "toolbar",
    "tree",
    "treeitem",
    "treegrid",
    "grid",
  ].forEach(function (role) {
    ACTION_COMPOSITE_ROLES[role] = true;
  });

  /** 容器 role：作为节点输出并继续向下遍历。 */
  var CONTAINER_ROLES = Object.create(null);
  [
    "dialog",
    "alertdialog",
    "list",
    "listitem",
    "table",
    "rowgroup",
    "row",
    "cell",
    "columnheader",
    "rowheader",
    "group",
    "navigation",
    "main",
    "region",
    "form",
    "search",
    "banner",
    "complementary",
    "contentinfo",
    "article",
    "figure",
    "feed",
    "status",
    "alert",
    "tooltip",
    "log",
    "marquee",
    "timer",
    "tabpanel",
    "separator",
    "img",
  ].forEach(function (role) {
    CONTAINER_ROLES[role] = true;
  });

  /** 文本字段的最大输出长度。 */
  var MAX_TEXT_LENGTH = 80;

  /** 使角采样点避开精确边界的缩进量。 */
  var SAMPLE_INSET = 1;

  /**
   * 将几何值舍入到一位小数。
   *
   * @param {number} value 原始 CSS 像素值。
   * @returns {number} 舍入到一位小数的值。
   */
  function round1(value) {
    return Math.round(value * 10) / 10;
  }

  /**
   * 将字符串截断为前 80 个 Unicode 码点。
   *
   * @param {string} value 已归一化、待截断的字符串。
   * @returns {string} 原字符串或其 80 码点前缀。
   */
  function clipText(value) {
    var chars = Array.from(value);
    return chars.length > MAX_TEXT_LENGTH
      ? chars.slice(0, MAX_TEXT_LENGTH).join("")
      : value;
  }

  /**
   * 报告元素自身是否将 contenteditable 声明为可编辑。
   *
   * 只有属性值为空或值为不区分大小写的 "true" 才算数；
   * 继承而来的可编辑状态通过 isContentEditable 单独处理。
   *
   * @param {Element} el 要检查的元素。
   * @returns {boolean} 当元素自身的 contenteditable 可编辑时为 true。
   */
  function hasOwnEditableAttribute(el) {
    if (!el.hasAttribute("contenteditable")) {
      return false;
    }
    var value = el.getAttribute("contenteditable");
    return value === "" || value.toLowerCase() === "true";
  }

  /**
   * 解析元素的 ARIA role，并在必要时根据其 tag 推断。
   *
   * @param {Element} el 要解析 role 的元素。
   * @param {string} tag 元素的大写（或 SVG 大小写形式）tag 名。
   * @returns {string|null} 显式或隐式的 role；没有时为 null。
   */
  function inferRole(el, tag) {
    var explicit = el.getAttribute("role");
    if (explicit !== null && explicit.trim() !== "") {
      return explicit.trim();
    }
    if ((tag === "A" || tag === "AREA") && el.hasAttribute("href")) {
      return "link";
    }
    if (tag === "BUTTON" || tag === "SUMMARY") {
      return "button";
    }
    if (tag === "INPUT") {
      var type = (el.getAttribute("type") || "").toLowerCase();
      if (type === "checkbox") {
        return "checkbox";
      }
      if (type === "radio") {
        return "radio";
      }
      if (type === "submit" || type === "button" || type === "image") {
        return "button";
      }
      if (type === "range") {
        return "slider";
      }
      return "textbox";
    }
    if (tag === "TEXTAREA") {
      return "textbox";
    }
    if (tag === "SELECT") {
      return "combobox";
    }
    if (tag === "OPTION") {
      return "option";
    }
    if (tag === "DIALOG") {
      return "dialog";
    }
    if (tag === "UL" || tag === "OL") {
      return "list";
    }
    if (tag === "LI") {
      return "listitem";
    }
    if (tag === "TABLE") {
      return "table";
    }
    if (tag === "TR") {
      return "row";
    }
    if (tag === "TD") {
      return "cell";
    }
    if (tag === "TH") {
      return "columnheader";
    }
    if (tag === "NAV") {
      return "navigation";
    }
    if (tag === "MAIN") {
      return "main";
    }
    if (tag === "FORM") {
      return "form";
    }
    if (tag === "IMG") {
      return "img";
    }
    if (
      tag === "SECTION" &&
      (el.hasAttribute("aria-label") || el.hasAttribute("aria-labelledby"))
    ) {
      return "region";
    }
    if (hasOwnEditableAttribute(el)) {
      return "textbox";
    }
    return null;
  }

  /**
   * 查找与 input、select 或 textarea 关联的标签文本。
   *
   * `label[for=id]` 引用优先于包裹该控件的标签。
   *
   * @param {Element} el 要为其查找标签的表单控件。
   * @returns {string} 去除首尾空白的标签文本；不存在时为空字符串。
   */
  function labelText(el) {
    if (el.id) {
      var labels = document.getElementsByTagName("label");
      for (var i = 0; i < labels.length; i += 1) {
        if (labels[i].getAttribute("for") === el.id) {
          var forText = (labels[i].textContent || "").trim();
          if (forText !== "") {
            return forText;
          }
        }
      }
    }
    var wrapping = el.closest ? el.closest("label") : null;
    if (wrapping) {
      return (wrapping.textContent || "").trim();
    }
    return "";
  }

  /**
   * 计算元素的无障碍名称。
   *
   * 按 aria-label、aria-labelledby、关联/包裹标签、placeholder、title、alt
   * 的顺序，第一个去除首尾空白后非空的候选者胜出。
   *
   * @param {Element} el 要计算名称的元素。
   * @param {string} tag 元素的 tag 名。
   * @returns {string} 去除首尾空白并截断到 80 码点的名称；没有时为 ""。
   */
  function computeName(el, tag) {
    var ariaLabel = el.getAttribute("aria-label");
    if (ariaLabel !== null && ariaLabel.trim() !== "") {
      return clipText(ariaLabel.trim());
    }

    var labelledby = el.getAttribute("aria-labelledby");
    if (labelledby !== null) {
      var referenced = [];
      labelledby.split(/\s+/).forEach(function (id) {
        if (id === "") {
          return;
        }
        var ref = document.getElementById(id);
        if (ref) {
          referenced.push(ref.textContent || "");
        }
      });
      var joined = referenced.join(" ").trim();
      if (joined !== "") {
        return clipText(joined);
      }
    }

    if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") {
      var associated = labelText(el);
      if (associated !== "") {
        return clipText(associated);
      }
    }

    var placeholder = el.getAttribute("placeholder");
    if (placeholder !== null && placeholder.trim() !== "") {
      return clipText(placeholder.trim());
    }

    var title = el.getAttribute("title");
    if (title !== null && title.trim() !== "") {
      return clipText(title.trim());
    }

    var alt = el.getAttribute("alt");
    if (alt !== null && alt.trim() !== "") {
      return clipText(alt.trim());
    }

    return "";
  }

  /**
   * 收集元素中位于白名单内且非空的属性。
   *
   * 当前输入的 `value` 永远不会被收集：本应用程序管理敏感数据，
   * 输入内容绝不能离开页面。
   *
   * @param {Element} el 要检查的元素。
   * @returns {Object} 属性名到去除首尾空白后属性值的映射；可能为空。
   */
  function collectAttributes(el) {
    var attrs = {};
    var attributes = el.attributes;
    for (var i = 0; i < attributes.length; i += 1) {
      var attr = attributes[i];
      var name = attr.name;
      var allowed = ATTR_WHITELIST[name] === true || name.indexOf("aria-") === 0;
      if (!allowed) {
        continue;
      }
      var value = attr.value.trim();
      if (value !== "") {
        attrs[name] = value;
      }
    }
    return attrs;
  }

  /**
   * 收集元素子树的可见文本。
   *
   * 只拼接渲染中且未被跳过元素内部的文本；不可见的子树不贡献任何内容。
   * 连续空白折叠为一个空格，结果截断到 80 码点。
   *
   * @param {Element} el 要读取的子树的根。
   * @returns {string} 归一化后的可见文本；没有时为空字符串。
   */
  function aggregatedText(el) {
    var parts = [];
    collectTextParts(el, parts);
    var joined = parts.join(" ").replace(/\s+/g, " ").trim();
    return joined === "" ? "" : clipText(joined);
  }

  /**
   * 将所有可见子孙节点的文本追加到 parts 数组。
   *
   * @param {Element} el 要遍历其子树的元素。
   * @param {string[]} parts 原始文本值的可变累加器。
   */
  function collectTextParts(el, parts) {
    var childNodes = el.childNodes;
    for (var i = 0; i < childNodes.length; i += 1) {
      var node = childNodes[i];
      if (node.nodeType === Node.TEXT_NODE) {
        if (node.nodeValue !== "") {
          parts.push(node.nodeValue);
        }
        continue;
      }
      if (node.nodeType !== Node.ELEMENT_NODE) {
        continue;
      }
      if (SKIP_TAGS[node.tagName] === true || !isRendered(node)) {
        continue;
      }
      collectTextParts(node, parts);
    }
  }

  /**
   * 测量文本节点的视口矩形。
   *
   * @param {Node} node 要测量的文本节点。
   * @returns {Object|null} 舍入后的 CSS bounds；矩形为空时为 null。
   */
  function textBounds(node) {
    var range = document.createRange();
    range.selectNodeContents(node);
    var rect = range.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) {
      return null;
    }
    return {
      left: round1(rect.left),
      top: round1(rect.top),
      width: round1(rect.width),
      height: round1(rect.height),
    };
  }

  /**
   * 报告元素是否可由用户编辑。
   *
   * @param {Element} el 要检查的元素。
   * @param {string|null} role 元素的已解析 role。
   * @returns {boolean} 可编辑 role 和 content-editable 元素为 true。
   */
  function isEditable(el, role) {
    return (role !== null && EDITABLE_ROLES[role] === true) || el.isContentEditable;
  }

  /**
   * 报告元素是否参与键盘 Tab 顺序，或本身是天生可交互的控件。
   *
   * @param {Element} el 要检查的元素。
   * @param {string} tag 元素的 tag 名。
   * @param {string|null} role 元素的已解析 role。
   * @returns {boolean} 元素可聚焦时为 true。
   */
  function isFocusable(el, tag, role) {
    if (el.tabIndex >= 0) {
      return true;
    }
    if (tag === "SELECT" || tag === "TEXTAREA" || tag === "INPUT") {
      return true;
    }
    if (el.isContentEditable) {
      return true;
    }
    if (role !== null && INTERACTIVE_ROLES[role] === true) {
      return true;
    }
    return false;
  }

  /**
   * 构建元素的状态标志。
   *
   * 只有当 aria-expanded 显式解析为 "true" 或 "false" 时才存在 `expanded`；
   * 其它所有状态始终会被报告。
   *
   * @param {Element} el 要检查的元素。
   * @returns {Object} 节点的 states 对象。
   */
  function collectStates(el) {
    var states = {
      disabled: el.disabled === true || el.getAttribute("aria-disabled") === "true",
      checked: el.checked === true || el.getAttribute("aria-checked") === "true",
      selected: el.selected === true || el.getAttribute("aria-selected") === "true",
    };
    var expanded = el.getAttribute("aria-expanded");
    if (expanded === "true") {
      states.expanded = true;
    } else if (expanded === "false") {
      states.expanded = false;
    }
    states.focused = document.activeElement === el;
    return states;
  }

  /**
   * 报告元素是否被渲染。
   *
   * CSS 的 visibility:hidden 与 display:none 通过 checkVisibility 检测，
   * 而 opacity 被有意忽略。尺寸为零的盒子在这里不被视为隐藏：
   * 诸如 Vuetify 的 `.v-overlay-container` 之类的框架悬浮层容器自身没有盒子，
   * 却承载着整个对话框子树，其条目会被提升到最近的保留祖先。
   * Chromium 会将同样没有盒子的 `display: contents` 元素报告为不可见；
   * 出于同样的原因，仍会遍历它们的子树。
   *
   * @param {Element} el 要测试的元素。
   * @returns {boolean} 当必须跳过该元素子树时为 false。
   */
  function isRendered(el) {
    if (typeof el.checkVisibility !== "function") {
      return true;
    }
    if (el.checkVisibility({ checkOpacity: false, checkVisibilityCSS: true }) !== false) {
      return true;
    }
    return window.getComputedStyle(el).display === "contents";
  }

  /**
   * 收集遮挡测试所用的采样点。
   *
   * 采样点集合由四个内缩的角点和 3x3 网格的中心点组成。
   * 角点用于证明内容非常满的容器仍有可见边缘，
   * 而网格中心点覆盖控件与文本的内部区域。
   * 视口之外的采样点会被丢弃，因为 elementFromPoint 无法解析它们。
   *
   * @param {Object} bounds 包含 left、top、width 和 height 的 bounds 对象。
   * @returns {Object[]} 视口坐标系中的采样点。
   */
  function samplePoints(bounds) {
    var inset = Math.min(SAMPLE_INSET, bounds.width / 4, bounds.height / 4);
    var left = bounds.left + inset;
    var right = bounds.left + bounds.width - inset;
    var top = bounds.top + inset;
    var bottom = bounds.top + bounds.height - inset;
    var stepX = (right - left) / 3;
    var stepY = (bottom - top) / 3;
    var points = [];
    function push(x, y) {
      if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight) {
        return;
      }
      points.push({ x: x, y: y });
    }

    push(left, top);
    push(right, top);
    push(left, bottom);
    push(right, bottom);
    for (var i = 0; i < 3; i += 1) {
      for (var j = 0; j < 3; j += 1) {
        push(left + stepX * (i + 0.5), top + stepY * (j + 0.5));
      }
    }
    return points;
  }

  /**
   * 报告视口中的一个点是否解析到元素自身的分支。
   *
   * 当该处最顶层元素是元素本身、其某个子孙节点或其某个祖先节点时，
   * 该点算作可达；对话框遮罩等无关的悬浮层节点都不匹配，
   * 而后者正是本测试要捕获的遮挡。
   *
   * @param {Element} el 被测元素。
   * @param {number} x 视口 x 坐标。
   * @param {number} y 视口 y 坐标。
   * @returns {boolean} 当该点未被无关节点覆盖时为 true。
   */
  function pointReachesElement(el, x, y) {
    var hit = document.elementFromPoint(x, y);
    if (hit === null) {
      return false;
    }
    return el === hit || el.contains(hit) || hit.contains(el);
  }

  /**
   * 报告视口中的一个点是否解析到文本节点的祖先链中。
   *
   * 文本没有自己的元素，因此只有当该处最顶层元素是其父元素或父元素的某个祖先时，
   * 该点才算作可达；兄弟节点或任何无关的悬浮层都算作覆盖。
   *
   * @param {Node} node 被测文本节点。
   * @param {number} x 视口 x 坐标。
   * @param {number} y 视口 y 坐标。
   * @returns {boolean} 当该点未被无关节点覆盖时为 true。
   */
  function pointReachesText(node, x, y) {
    var hit = document.elementFromPoint(x, y);
    if (hit === null) {
      return false;
    }
    var current = node.parentElement;
    while (current !== null) {
      if (current === hit) {
        return true;
      }
      current = current.parentElement;
    }
    return false;
  }

  /**
   * 报告 bounds 矩形是否存在未被遮挡的采样点。
   *
   * 视口内没有任何采样点的矩形会被保留：完全滚动到视口外的内容
   * 绝不能与内容被遮挡相混淆。
   *
   * @param {Object} bounds 包含 left、top、width 和 height 的 bounds 对象。
   * @param {function(number, number): boolean} reaches 解析单个采样点的
   *   点可达性谓词。
   * @returns {boolean} 当至少一个采样点未被遮挡时为 true。
   */
  function isUnoccluded(bounds, reaches) {
    var points = samplePoints(bounds);
    if (points.length === 0) {
      return true;
    }
    for (var i = 0; i < points.length; i += 1) {
      if (reaches(points[i].x, points[i].y)) {
        return true;
      }
    }
    return false;
  }

  /**
   * 构建元素矩形的 bounds 对象。
   *
   * @param {DOMRect} rect 以 CSS 像素表示的元素矩形。
   * @returns {Object} 舍入后的 bounds 对象。
   */
  function elementBounds(rect) {
    return {
      left: round1(rect.left),
      top: round1(rect.top),
      width: round1(rect.width),
      height: round1(rect.height),
    };
  }

  /**
   * 对元素进行分类，以用于简化输出。
   *
   * @param {string|null} role 元素的已解析 role。
   * @param {boolean} editable 元素是否接受可编辑文本。
   * @param {boolean} focusable 元素是否可聚焦。
   * @returns {string} `action-leaf`、`action-composite`、`container`
   *   或 `layout` 之一。
   */
  function classifyElement(role, editable, focusable) {
    if (role !== null) {
      if (ACTION_LEAF_ROLES[role] === true) {
        return "action-leaf";
      }
      if (ACTION_COMPOSITE_ROLES[role] === true) {
        return "action-composite";
      }
      // 已知容器和未知 role 都会保持其内容可见。
      return "container";
    }
    if (editable || focusable) {
      return "action-leaf";
    }
    return "layout";
  }

  /**
   * 构建单个元素的节点 JSON，不包含其 `children`。
   *
   * @param {Element} el 要转换的元素。
   * @param {string} tag 元素的 tag 名。
   * @param {string|null} role 元素的已解析 role。
   * @param {boolean} editable 元素是否接受可编辑文本。
   * @param {boolean} focusable 元素是否可聚焦。
   * @param {DOMRect} rect 以 CSS 像素表示的元素矩形。
   * @returns {Object} 节点的 JSON。
   */
  function buildEntry(el, tag, role, editable, focusable, rect) {
    var entry = { tag: tag };
    if (role !== null) {
      entry.role = role;
    }
    var name = computeName(el, tag);
    if (name !== "") {
      entry.name = name;
    }
    if (editable) {
      entry.editable = true;
    }
    if (focusable) {
      entry.focusable = true;
    }
    entry.bounds = elementBounds(rect);
    entry.states = collectStates(el);
    var attrs = collectAttributes(el);
    if (Object.keys(attrs).length > 0) {
      entry.attrs = attrs;
    }
    return entry;
  }

  /**
   * 收集元素所有子节点的简化条目。
   *
   * 文本节点变为用 DOM Range 测量的 `#text` 条目；
   * 元素子节点按文档顺序递归转换。
   * insideTooltip 为 true 时不施加遮挡判定：role=tooltip 的悬浮子树在可见期间
   * 内容为 pointer-events:none，会被遮挡测试误判；未显示时其内容 display:none 仍不入树。
   *
   * @param {Element} el 要转换其子节点的元素。
   * @param {boolean} insideTooltip 当前是否位于 role=tooltip 的悬浮子树内。
   * @returns {Object[]} 子条目；可能为空。
   */
  function collectChildren(el, insideTooltip) {
    var entries = [];
    var childNodes = el.childNodes;
    for (var i = 0; i < childNodes.length; i += 1) {
      var node = childNodes[i];
      if (node.nodeType === Node.TEXT_NODE) {
        var text = (node.nodeValue || "").replace(/\s+/g, " ").trim();
        if (text === "") {
          continue;
        }
        var bounds = textBounds(node);
        if (bounds === null) {
          continue;
        }
        if (
          !insideTooltip &&
          !isUnoccluded(bounds, function (x, y) {
            return pointReachesText(node, x, y);
          })
        ) {
          continue;
        }
        entries.push({
          tag: "#text",
          bounds: bounds,
          text: clipText(text),
        });
        continue;
      }
      if (node.nodeType !== Node.ELEMENT_NODE) {
        continue;
      }
      var childEntries = collectElement(node, insideTooltip);
      for (var j = 0; j < childEntries.length; j += 1) {
        entries.push(childEntries[j]);
      }
    }
    return entries;
  }

  /**
   * 收集元素贡献的简化条目。
   *
   * 仅用于布局的元素不贡献节点：其子孙节点会被提升到最近的保留祖先，
   * 这也是结果为数组的原因。可拖拽元素与携带连接桩数据属性的元素例外：
   * 它们是交互对象（如拖拽手柄、连接桩），没有语义 role 也保留为容器输出，
   * 以便自动化定位其交互位置。action leaf 不输出其内部子树，
   * 并携带聚合后的可见文本。容器和复合 Widget 会连同其子节点一起输出。
   * 既无内容又无名称的条目会被丢弃：空容器和无名称的零尺寸控件
   * （其中包括不可见的悬浮层残留）只会增加噪声；可拖拽元素与连接桩元素
   * 即使没有名称与子节点也保留，供自动化定位交互位置。
   *
   * 这里会应用两道可见性过滤器。标记了 `aria-hidden="true"` 或 `inert`
   * 的子树会被跳过，因为无障碍树会隐藏它们（可拖拽元素与携带连接桩数据
   * 属性的元素除外，它们是显式的交互对象）。其余条目必须证明至少存在
   * 一个未被遮挡的采样点，因此对话框遮罩或任何其它悬浮层背后的内容也会被丢弃。
   * role=tooltip 的悬浮子树在可见期间不参与遮挡判定（其内容为 pointer-events:none）；
   * 未显示时其内容 display:none 仍不入树。
   *
   * @param {Element} el 要转换的元素。
   * @param {boolean} insideTooltip 当前是否位于 role=tooltip 的悬浮子树内。
   * @returns {Object[]} 按文档顺序排列的零个或多个条目。
   */
  function collectElement(el, insideTooltip) {
    var tag = el.tagName;
    // 可拖拽元素与携带连接桩数据的元素是交互对象：即使被标记为装饰性隐藏
    // （如 Vuetify 图标默认的 aria-hidden）或没有语义 role，也必须保留，
    // 否则拖拽手柄与连接桩无法被自动化定位。
    var interactiveLayout =
      el.draggable === true ||
      el.hasAttribute("data-handleid") ||
      el.hasAttribute("data-handlepos");
    if (
      SKIP_TAGS[tag] === true ||
      !isRendered(el) ||
      (el.getAttribute("aria-hidden") === "true" && !interactiveLayout) ||
      el.hasAttribute("inert")
    ) {
      return [];
    }

    var role = inferRole(el, tag);
    var tooltipScope = insideTooltip || role === "tooltip";
    var editable = isEditable(el, role);
    var focusable = isFocusable(el, tag, role);
    var category = classifyElement(role, editable, focusable);

    if (category === "layout" && !interactiveLayout) {
      return collectChildren(el, tooltipScope);
    }

    var rect = el.getBoundingClientRect();
    var entry = buildEntry(el, tag, role, editable, focusable, rect);
    if (
      !tooltipScope &&
      !isUnoccluded(entry.bounds, function (x, y) {
        return pointReachesElement(el, x, y);
      })
    ) {
      return [];
    }

    if (category === "action-leaf") {
      var text = aggregatedText(el);
      if (text !== "") {
        entry.text = text;
      }
      if (
        (entry.bounds.width === 0 || entry.bounds.height === 0) &&
        entry.name === undefined &&
        entry.text === undefined
      ) {
        return [];
      }
      return [entry];
    }

    var children = collectChildren(el, tooltipScope);
    if (children.length === 0 && entry.name === undefined && !interactiveLayout) {
      return [];
    }
    if (children.length > 0) {
      entry.children = children;
    }
    return [entry];
  }

  window.__iNetDebugAutomation = {
    /**
     * 同步收集简化后的 UI 树。
     *
     * 这里的 bounds 保持为相对于视口的 CSS 像素；Rust 侧在通过 HTTP 返回载荷之前
     * 会将其换算为窗口物理像素，因此调用方从不接触 CSS 坐标系。
     * role=tooltip 的悬浮子树在可见期间不参与遮挡判定；未显示时其内容不入树。
     * 遍历在页面状态异常时可能抛出异常；注入的调用方会在 try/catch 中包装此调用，
     * 并通过求值结果传递该失败。
     *
     * @returns {Object[]} 以按文档顺序排列的根条目数组表示的简化树。
     */
    collect: function () {
      return collectChildren(document.documentElement, false);
    },
  };
})();
