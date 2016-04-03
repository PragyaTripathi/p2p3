(function(app) {
  document.addEventListener('DOMContentLoaded', function() {
    ng.platform.browser.bootstrap(app.AceComponent);
  });
})(window.app || (window.app = {}));
