var sock = new WebSocket("ws://127.0.0.1:4242/");

sock.onopen = function(event){
  //TODO: connection is now open
};

sock.onmessage = function(event){
  var json = event.data,
  obj = eval("(" + json + ')');
  console.log(obj);
  switch (obj.variant) {
    case "InsertString":
      editor.setValue(obj.fields[1]); //TODO
      break;
    case "Output":
      console.log(obj.fields[0]);
      var output = document.getElementById('output');
      output.innerHTML = obj.fields[0];
      break;
    case "InsertChar":
      console.log(obj);
      var delta = asDelta(obj.fields[1], true, obj.fields[0]);
      editor.getSession().getDocument().applyDeltas([delta]);
      break;
    case "DeleteChar":

      break;
    default:

  }
}


// Convert a WOOT operation to an ACE delta object for WOOT index i:
function asDelta(ch, isVisible, i) {
  return {
    action: isVisible ? "insertText" : "removeText",
    range: {
      start: pos(i),
      end:   pos(i+1)
    },
    text: ch
  };
}

function compileOnClick() {
    sock.send(JSON.stringify({
      variant: "Compile",
      fields: [],
    }));
}
