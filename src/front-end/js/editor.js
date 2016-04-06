var editor = ace.edit("editor");
editor.setTheme("ace/theme/merbivore");
editor.getSession().getDocument().setNewLineMode("unix");
editor.getSession().setMode("ace/mode/markdown");
