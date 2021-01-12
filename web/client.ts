import { Terminal } from "xterm";
import { wsurl } from "./config";

let loading_element = document.querySelector(".loading");

Promise.all([import("xterm"), import("xterm-addon-fit"), import("./theme")]).then(([xterm, xtermfit, theme]) => {
  loading_element.remove();
  let term = new Terminal();
  let fit = new xtermfit.FitAddon();
  term.loadAddon(fit);
  if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    // todo
  } else {
    term.setOption("theme", theme.light);
  }
  term.setOption("drawBoldTextInBrightColors", false);
  term.setOption("fontFamily", "'Inconsolata for Powerline', 'Hack', monospace");
  let term_container = document.createElement("div");
  term_container.classList.add("term-container");
  document.body.appendChild(term_container);
  term.open(term_container);

  function resize() {
    fit.fit();
  }

  resize();
  let last_resize_handle = null;
  function debounced_resize() {
    if (last_resize_handle !== null) {
      clearTimeout(last_resize_handle);
      last_resize_handle = null;
    }
    last_resize_handle = setTimeout(() => resize(), 100);
  }
  window.addEventListener("resize", evt => {
    debounced_resize();
  });

  let ws = new WebSocket(wsurl);
  ws.addEventListener("message", evt => {
    console.log(evt.data);
    term.write(evt.data);
  });

  function send_size(cols, rows) {
    let data = new ArrayBuffer(5);
    let u8s = new Uint8Array(data);
    u8s[0] = 1;
    if (cols > 0xffff) {
      cols = 0xffff;
    }
    if (rows > 0xffff) {
      rows = 0xffff;
    }
    u8s[1] = cols >> 8;
    u8s[2] = cols & 0xff;
    u8s[3] = rows >> 8;
    u8s[4] = rows & 0xff;
    ws.send(data);
  }

  let closed = false;
  ws.addEventListener("open", evt => {
    term.onData(data => {
      if (!closed) {
        ws.send(data);
      }
    });
    send_size(term.cols, term.rows);
    term.onResize(({ cols, rows }) => {
      if (!closed) {
        send_size(cols, rows);
      }
    });
  });
  ws.addEventListener("error", evt => {
    if (!closed) {
      term.write("\x1b[0;31;1m\n\rWebsoekct connection broke. Refresh to reconnect.");
    }
    closed = true;
  });
  ws.addEventListener("close", evt => {
    if (!closed) {
      term.write("\x1b[0;31;1m\n\rServer closed our connection. Refresh to reconnect.");
    }
    closed = true;
  });

});
