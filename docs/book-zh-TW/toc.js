// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="index.html"><strong aria-hidden="true">1.</strong> 介紹</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="intro/overview.html"><strong aria-hidden="true">1.1.</strong> 什麼是 Sylvia-IoT？</a></li><li class="chapter-item expanded "><a href="intro/concept.html"><strong aria-hidden="true">1.2.</strong> 概念</a></li></ol></li><li class="chapter-item expanded "><a href="guide/index.html"><strong aria-hidden="true">2.</strong> 使用指南</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="guide/quick.html"><strong aria-hidden="true">2.1.</strong> 快速開始</a></li><li class="chapter-item expanded "><a href="guide/configuration.html"><strong aria-hidden="true">2.2.</strong> 設定檔</a></li></ol></li><li class="chapter-item expanded "><a href="arch/index.html"><strong aria-hidden="true">3.</strong> 內部架構</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="arch/arch.html"><strong aria-hidden="true">3.1.</strong> 架構</a></li><li class="chapter-item expanded "><a href="arch/flow.html"><strong aria-hidden="true">3.2.</strong> 資料流</a></li><li class="chapter-item expanded "><a href="arch/cache.html"><strong aria-hidden="true">3.3.</strong> 快取</a></li></ol></li><li class="chapter-item expanded "><a href="dev/index.html"><strong aria-hidden="true">4.</strong> 開發指南</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dev/oauth2.html"><strong aria-hidden="true">4.1.</strong> OAuth2 認證</a></li><li class="chapter-item expanded "><a href="dev/network.html"><strong aria-hidden="true">4.2.</strong> 網路服務</a></li><li class="chapter-item expanded "><a href="dev/application.html"><strong aria-hidden="true">4.3.</strong> 應用服務</a></li><li class="chapter-item expanded "><a href="dev/core.html"><strong aria-hidden="true">4.4.</strong> Sylvia-IoT 核心</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="dev/dir.html"><strong aria-hidden="true">4.4.1.</strong> 目錄結構</a></li><li class="chapter-item expanded "><a href="dev/style.html"><strong aria-hidden="true">4.4.2.</strong> 程式碼風格</a></li><li class="chapter-item expanded "><a href="dev/testing.html"><strong aria-hidden="true">4.4.3.</strong> 撰寫測試</a></li></ol></li><li class="chapter-item expanded "><a href="dev/cross.html"><strong aria-hidden="true">4.5.</strong> 跨平台編譯</a></li></ol></li><li class="chapter-item expanded "><a href="appendex/index.html"><strong aria-hidden="true">5.</strong> 附錄</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="appendex/source.html"><strong aria-hidden="true">5.1.</strong> 資料來源</a></li><li class="chapter-item expanded "><a href="appendex/repo.html"><strong aria-hidden="true">5.2.</strong> 輔助專案</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString();
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
