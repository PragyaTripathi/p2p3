var sock = new WebSocket("ws://127.0.0.1:4242/");

sock.onopen = function(event){
  //TODO: connection is now open
};

sock.onmessage = function(event){
  var json = event.data,
  obj = eval("(" + json + ')');
  console.log(obj);
  switch (obj.variant) {
    case "Insert":
      editor.setValue(obj.fields[1]); //TODO
      break;
    case "Output":
      console.log(obj.fields[0]);
      var output = document.getElementById('output');
      output.innerHTML = obj.fields[0];
      break;
    default:

  }
}

function compileOnClick() {
    sock.send(JSON.stringify({
      variant: "Compile",
      fields: [],
    }));
}
