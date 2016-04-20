var sock = new WebSocket("ws://127.0.0.1:4242/");

sock.onopen = function(event){
  //TODO: connection is now open
};

sock.onmessage = function(event){
  console.log(event.data);
  var json = event.data,
  obj = eval("(" + json + ')');
  console.log(obj);
  editor.setValue(obj.fields[1]);
}

function compileOnClick() {
    console.log("Compile on click");
    sock.send(JSON.stringify({
      variant: "Compile",
      fields: [],
    }));
}
