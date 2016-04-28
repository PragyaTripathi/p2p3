var editor = ace.edit("editor");
editor.setTheme("ace/theme/cobalt");
editor.getSession().getDocument().setNewLineMode("unix");
editor.getSession().setMode("ace/mode/markdown");

// Convert ACE "position" (row/column) to WOOT "index":
function idx(position) {
  return editor.getSession().getDocument().positionToIndex(position);
}

// Covert a WOOT "index" into an ACE "position"
function pos(index) {
  return editor.getSession().getDocument().indexToPosition(index);
}

var marker = {};
marker.cursors = [];
marker.update = function(html, markerLayer, session, config) {
    var start = config.firstRow, end = config.lastRow;
    var cursors = this.cursors
    // for (var i = 0; i < cursors.length; i++) {
    for (var i in cursors) {
        var pos = cursors[i];
        if (pos.row < start) {
            continue
        } else if (pos.row > end) {
            break
        } else {
            // compute cursor position on screen
            // this code is based on ace/layer/marker.js
            var screenPos = session.documentToScreenPosition(pos)

            var height = config.lineHeight;
            var width = config.characterWidth;
            var top = markerLayer.$getTop(screenPos.row, config);
            var left = markerLayer.$padding + screenPos.column * width;
            // can add any html here
            html.push(
                "<div class='MyCursorClass' style='",
                "height:", height, "px;",
                "top:", top, "px;",
                "left:", left, "px; width:", width, "px'></div>"
            );
        }
    }
}
marker.redraw = function() {
   this.session._signal("changeFrontMarker");
}
marker.updateCursorPos = function(peerId, cursor) {
    // add to this cursors
    this.cursors[peerId] = cursor;
    console.log(peerId);
    console.log(this.cursors[peerId]);
    // trigger redraw
    marker.redraw();
}

marker.removeCursor = function(peerId) {
    console.log("removing func");
    console.log(peerId);
    console.log(this.cursors[peerId]);
    delete this.cursors[peerId];
    // trigger redraw
    marker.redraw();
}
marker.session = editor.session;
marker.session.addDynamicMarker(marker, true);
