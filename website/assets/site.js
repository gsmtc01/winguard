// WinGuard 웹사이트 공통 스크립트. 테마 토글, 모바일 메뉴, 목록 필터, 공지 상세.
// 테마 초기값은 <head> 인라인 스크립트가 이미 적용해 둔다(FOUC 방지).
(function () {
  "use strict";

  var STORAGE_KEY = "winguard-theme";
  var root = document.documentElement;

  // ── 테마 토글 ───────────────────────────────────────────
  var toggle = document.querySelector("[data-theme-toggle]");
  if (toggle) {
    // 초기 상태는 head 인라인 스크립트가 정한 테마를 따른다.
    toggle.setAttribute("aria-pressed", String(root.dataset.theme === "dark"));

    toggle.addEventListener("click", function () {
      var next = root.dataset.theme === "dark" ? "light" : "dark";
      root.dataset.theme = next;
      try {
        localStorage.setItem(STORAGE_KEY, next);
      } catch (e) {
        /* 프라이빗 모드 등에서 저장 실패는 무시 */
      }
      toggle.setAttribute("aria-pressed", String(next === "dark"));
    });
  }

  // ── 모바일 메뉴 ─────────────────────────────────────────
  var menuBtn = document.querySelector("[data-menu-toggle]");
  var menu = document.querySelector("[data-mobile-menu]");
  if (menuBtn && menu) {
    menuBtn.addEventListener("click", function () {
      var open = menu.dataset.open !== "true";
      menu.dataset.open = String(open);
      menuBtn.setAttribute("aria-expanded", String(open));
    });
  }

  // ── 스크린샷 슬롯 ───────────────────────────────────────
  // 파일이 아직 없으면 이미지를 숨겨 placeholder 문구를 노출한다.
  var shots = document.querySelectorAll(".shot img");
  for (var i = 0; i < shots.length; i++) {
    (function (img) {
      img.addEventListener("error", function () {
        img.hidden = true;
      });
      if (img.complete && img.naturalWidth === 0) img.hidden = true;
    })(shots[i]);
  }

  // ── 공통 헬퍼 ───────────────────────────────────────────

  function esc(s) {
    return String(s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  /** 칩/사이드 버튼 그룹에서 하나만 눌린 상태로 만든다. */
  function selectOnly(group, target) {
    for (var i = 0; i < group.length; i++) {
      group[i].setAttribute("aria-pressed", String(group[i] === target));
    }
  }

  var notices = window.WG_NOTICES || [];

  // ── 공지사항 목록 (notices.html) ────────────────────────
  var noticeList = document.querySelector("[data-notice-list]");
  if (noticeList) {
    var filterBtns = document.querySelectorAll("[data-notice-filter]");
    var pager = document.querySelector("[data-pager]");

    var renderNotices = function (cat) {
      var shown = notices.filter(function (n) {
        return cat === "전체" || n.category === cat;
      });

      if (shown.length === 0) {
        noticeList.innerHTML =
          '<p class="empty">해당 분류의 공지사항이 아직 없습니다.</p>';
        if (pager) pager.hidden = true;
        return;
      }

      noticeList.innerHTML = shown
        .map(function (n) {
          return (
            '<a class="notice-card" data-pinned="' +
            n.pinned +
            '" href="notice.html?id=' +
            n.id +
            '">' +
            '<span class="notice-card__head">' +
            '<span class="badge" data-cat="' +
            esc(n.category) +
            '">' +
            esc(n.category) +
            "</span>" +
            (n.pinned ? '<span class="badge badge--solid">고정</span>' : "") +
            '<span class="notice-card__title">' +
            esc(n.title) +
            "</span>" +
            "</span>" +
            '<span class="notice-card__date num">' +
            esc(n.date) +
            "</span>" +
            '<span class="notice-card__preview">' +
            esc(n.preview) +
            "</span>" +
            "</a>"
          );
        })
        .join("");
      if (pager) pager.hidden = false;
    };

    for (var f = 0; f < filterBtns.length; f++) {
      (function (btn) {
        btn.addEventListener("click", function () {
          selectOnly(filterBtns, btn);
          renderNotices(btn.dataset.noticeFilter);
        });
      })(filterBtns[f]);
    }

    renderNotices("전체");
  }

  // ── 공지 상세 (notice.html) ─────────────────────────────
  var article = document.querySelector("[data-article]");
  if (article) {
    var id = Number(new URLSearchParams(location.search).get("id"));
    var idx = notices.findIndex(function (n) {
      return n.id === id;
    });
    var current = idx >= 0 ? notices[idx] : null;

    if (!current) {
      article.innerHTML =
        '<h1>공지사항을 찾을 수 없습니다</h1>' +
        '<p class="empty">주소가 잘못되었거나 삭제된 글입니다.</p>';
      // 이전/다음 링크만 감추고 "목록으로" 버튼은 남긴다.
      var adjacent = document.querySelector("[data-adjacent] .adjacent");
      if (adjacent) adjacent.hidden = true;
    } else {
      document.title = current.title + " | WinGuard 공지사항";
      article.innerHTML =
        '<p class="article__meta">' +
        '<span class="badge" data-cat="' +
        esc(current.category) +
        '">' +
        esc(current.category) +
        "</span>" +
        '<span class="num">' +
        esc(current.date) +
        "</span>" +
        "</p>" +
        "<h1>" +
        esc(current.title) +
        "</h1>" +
        '<div class="article__body">' +
        current.body
          .map(function (p) {
            return "<p>" + esc(p) + "</p>";
          })
          .join("") +
        "</div>";

      // 이전/다음은 목록에 보이는 순서(notices.js 배열 순서)를 그대로 따른다.
      var fill = function (sel, item) {
        var row = document.querySelector(sel);
        if (!row) return;
        var title = row.querySelector(".adjacent__title");
        if (item) {
          row.href = "notice.html?id=" + item.id;
          title.textContent = item.title;
        } else {
          row.removeAttribute("href");
          row.setAttribute("aria-disabled", "true");
        }
      };
      fill("[data-prev]", notices[idx + 1]);
      fill("[data-next]", notices[idx - 1]);
    }
  }

  // ── Q&A 검색 · 분류 (qna.html) ──────────────────────────
  var faqList = document.querySelector("[data-faq-list]");
  if (faqList) {
    var faqs = faqList.querySelectorAll(".faq");
    var catBtns = document.querySelectorAll("[data-faq-cat]");
    var searchInput = document.querySelector("[data-faq-search]");
    var emptyMsg = document.querySelector("[data-faq-empty]");
    var activeCat = "전체";

    var applyFaqFilter = function () {
      var q = searchInput ? searchInput.value.trim().toLowerCase() : "";
      var hits = 0;
      for (var i = 0; i < faqs.length; i++) {
        var el = faqs[i];
        var catOk = activeCat === "전체" || el.dataset.cat === activeCat;
        var textOk = q === "" || el.textContent.toLowerCase().indexOf(q) !== -1;
        var show = catOk && textOk;
        el.hidden = !show;
        if (show) hits++;
      }
      if (emptyMsg) emptyMsg.hidden = hits > 0;
    };

    for (var c = 0; c < catBtns.length; c++) {
      (function (btn) {
        btn.addEventListener("click", function () {
          selectOnly(catBtns, btn);
          activeCat = btn.dataset.faqCat;
          applyFaqFilter();
        });
      })(catBtns[c]);
    }

    if (searchInput) searchInput.addEventListener("input", applyFaqFilter);

    // 자주 찾는 질문 칩: 필터를 초기화하고 해당 항목을 펼친다.
    var quickBtns = document.querySelectorAll("[data-faq-open]");
    for (var qi = 0; qi < quickBtns.length; qi++) {
      (function (btn) {
        btn.addEventListener("click", function () {
          var target = document.getElementById(btn.dataset.faqOpen);
          if (!target) return;
          activeCat = "전체";
          selectOnly(catBtns, catBtns[0]);
          if (searchInput) searchInput.value = "";
          applyFaqFilter();
          target.open = true;
          target.scrollIntoView({ block: "center", behavior: "smooth" });
        });
      })(quickBtns[qi]);
    }
  }
})();
