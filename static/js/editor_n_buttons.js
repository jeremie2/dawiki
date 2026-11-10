/*
You can find comprehensive documentation of the
following functions into doc/js_into_editor.md
*/

// :::::::::::::::::::::: INIT

const markdownInput = document.getElementById('markdown-input');
const preview = document.getElementById('preview');

function updatePreview() {
    preview.innerHTML = marked.parse(markdownInput.value);

    generateTableOfContents(preview);
    parseUbuntuVersions(preview);
    parseForumTag(preview);

    preview.innerHTML = DOMPurify.sanitize(preview.innerHTML);

    if (typeof DOMPurify !== "undefined") {
        preview.innerHTML = DOMPurify.sanitize(preview.innerHTML);
    }

    if (typeof Prism !== "undefined") {
        Prism.highlightAllUnder(preview);
    }
}

markdownInput.addEventListener('input', updatePreview);

markdownInput.value = '# Benvenuto!\n\nInsercisci del testo...';
updatePreview();

// :::::::::::::::::::::: HELPER FOR INPUT WITH UNDO (Ctrl+z)

function execInsert(newText, start, end) {
  markdownInput.setSelectionRange(start, end);
  try {
    document.execCommand('insertText', false, newText);
  } catch (e) {
    const text = markdownInput.value;
    markdownInput.value = text.substring(0, start) + newText + text.substring(end);
  }
}

// :::::::::::::::::::::: SYMMETRICAL SYMBOLS (bold, italic, tt...)

function insertMarkdown(startSymbol, endSymbol = startSymbol, placeholder = "") {
    markdownInput.focus();
    const start = markdownInput.selectionStart;
    const end = markdownInput.selectionEnd;
    const text = markdownInput.value;
    const selected = text.substring(start, end);

    let newText = "";
    let selectionOffsetStart = 0;
    let selectionOffsetEnd = 0;

    // Toggle OFF
    if (selected.startsWith(startSymbol) && selected.endsWith(endSymbol)) {
        newText = selected.substring(startSymbol.length, selected.length - endSymbol.length);
        selectionOffsetStart = start;
        selectionOffsetEnd = start + newText.length;
    } 
    // Toggle ON
    else {
        const defaultText = selected || placeholder;
        newText = startSymbol + defaultText + endSymbol;
        
        selectionOffsetStart = start;
        selectionOffsetEnd = start + newText.length;
    }

    execInsert(newText, start, end);
    markdownInput.setSelectionRange(selectionOffsetStart, selectionOffsetEnd);
    updatePreview();
}

// :::::::::::::::::::::: SINGLE SYMBOLS (heading, quote...)

function toggleLineSymbol(symbol) {
  markdownInput.focus();

  const start = markdownInput.selectionStart;
  const end = markdownInput.selectionEnd;
  const text = markdownInput.value;

  const selectionStartLineStart = text.lastIndexOf('\n', start - 1) + 1;
  let selectionEndLineEnd = text.indexOf('\n', end);
  if (selectionEndLineEnd === -1) selectionEndLineEnd = text.length;

  const fullLinesText = text.substring(selectionStartLineStart, selectionEndLineEnd);
  const lines = fullLinesText.split('\n');

  const isRemoving = lines[0].startsWith(symbol);
  
  const updatedLines = lines.map(line => {
    if (isRemoving) {
      return line.startsWith(symbol) ? line.substring(symbol.length) : line;
    } else {
      return symbol + line;
    }
  });

  const newLinesText = updatedLines.join('\n');

  execInsert(newLinesText, selectionStartLineStart, selectionEndLineEnd);

  // Only cursor case.
  if (start === end) {
    const delta = isRemoving ? -symbol.length : symbol.length;
    const newCursorPos = Math.max(selectionStartLineStart, start + delta);
    markdownInput.setSelectionRange(newCursorPos, newCursorPos);
  } else {
    // Text selection case.
    markdownInput.setSelectionRange(selectionStartLineStart, selectionStartLineStart + newLinesText.length);
  }

  updatePreview();
}

// :::::::::::::::::::::: CODE BLOCKS

function toggleCodeBlock() {
  markdownInput.focus();

  const start = markdownInput.selectionStart;
  const end = markdownInput.selectionEnd;
  const text = markdownInput.value;
  const selected = text.substring(start, end);

  let newText = "";
  let selectionOffsetStart = 0;
  let selectionOffsetEnd = 0;

  const isCodeBlock = selected.startsWith("```\n") && selected.endsWith("\n```");

  if (isCodeBlock) {
    newText = selected.substring(4, selected.length - 4);
    selectionOffsetStart = start;
    selectionOffsetEnd = start + newText.length;
  } else {
    const defaultText = selected || "scrivi qui il codice";
    newText = "```\n" + defaultText + "\n```";
    selectionOffsetStart = start + 4;
    selectionOffsetEnd = start + 4 + defaultText.length;
  }

  execInsert(newText, start, end);
  markdownInput.setSelectionRange(selectionOffsetStart, selectionOffsetEnd);
  updatePreview();
}

// :::::::::::::::::::::: NOTE, WARNING...

function insertNote(header, message) {
  markdownInput.focus();
  const start = markdownInput.selectionStart;
  const end = markdownInput.selectionEnd;
  const text = header + message
  
  execInsert(text, start, end);
  
  const selectStart = start + header.length;
  markdownInput.setSelectionRange(selectStart, selectStart + message.length);
  updatePreview();
}

// :::::::::::::::::::::: EASY INSERT
// No need for text selection at the end.
// It's used for tables.

function easyInsert(template) {
  markdownInput.focus();
  const start = markdownInput.selectionStart;
  const end = markdownInput.selectionEnd;
  const text = template;
  
  execInsert(text, start, end);
  updatePreview();
}

// :::::::::::::::::::::: EVENT LISTENERS

document.getElementById('btn-bold').addEventListener('click', () => insertMarkdown('**', '**', ''));
document.getElementById('btn-italic').addEventListener('click', () => insertMarkdown('*', '*', ''));
document.getElementById('btn-tt').addEventListener('click', () => insertMarkdown('`', '`', ''));
document.getElementById('btn-codeblock').addEventListener('click', toggleCodeBlock);
document.getElementById('btn-br').addEventListener('click', () => insertMarkdown('', '<br>', ''));
document.getElementById('btn-comment').addEventListener('click', () => insertMarkdown('<!--', '-->', 'testo'));

document.getElementById('btn-bullet').addEventListener('click', () => toggleLineSymbol('- '));
document.getElementById('btn-num').addEventListener('click', () => toggleLineSymbol('1. '));
document.getElementById('btn-quote').addEventListener('click', () => toggleLineSymbol('> '));

document.getElementById('btn-h1').addEventListener('click', () => toggleLineSymbol('# '));
document.getElementById('btn-h2').addEventListener('click', () => toggleLineSymbol('## '));
document.getElementById('btn-h3').addEventListener('click', () => toggleLineSymbol('### '));

document.getElementById('btn-link').addEventListener('click', () => {
    markdownInput.focus();
    const start = markdownInput.selectionStart;
    const end = markdownInput.selectionEnd;
    const selected = markdownInput.value.substring(start, end);
    
    const label = selected || 'Testo';
    const template = `[${label}](https://example.com)`;
    
    execInsert(template, start, end);
    
    // The URL example will be immediately selected to 
    // change it quickly.
    const offsetStart = start + label.length + 3;
    markdownInput.setSelectionRange(offsetStart, offsetStart + 19);
    updatePreview();
});

document.getElementById('btn-anchor').addEventListener('click', () => insertMarkdown('', '<a id="nome_anchor"></a>', ''));

document.getElementById('btn-img').addEventListener('click', () => {
    markdownInput.focus();
    const start = markdownInput.selectionStart;
    const end = markdownInput.selectionEnd;
    
    const template = '\n<div align="center">\n  <img src="img.png" width="400">\n</div>\n';
    execInsert(template, start, end);
    
    // The `img.png` string will be immediately selected to
    // change it quickly. 
    const offsetStart = start + 34;
    markdownInput.setSelectionRange(offsetStart, offsetStart + 7);
    updatePreview();
});
document.getElementById('btn-table').addEventListener('click', () => {
  const template = '\n| Header 1 | Header 2 | Header 3 |\n| :---     | :---:    | ---:     |\n| Cella    | Cella    | Cella    |\n';
  easyInsert(template);
});
document.getElementById('btn-ubu-version').addEventListener('click', () => {
  const template = '\n[UBU ]\n';
  easyInsert(template);
});
document.getElementById('btn-link-forum').addEventListener('click', () => {
  const template = '\n[FORUM https://forum.ubuntu-it.org/viewtopic.php?t=641900]\n';
  easyInsert(template);
});
document.getElementById('btn-menu-1').addEventListener('click', () => {
  const template = '[IND]\n';
  easyInsert(template);
});
document.getElementById('btn-menu-2').addEventListener('click', () => {
  const template = '[IND-2]\n';
  easyInsert(template);
});
document.getElementById('btn-menu-3').addEventListener('click', () => {
  const template = '[IND-3]\n';
  easyInsert(template);
});
document.getElementById('btn-note').addEventListener('click', () => {
    const header = '\n> 📝 **Nota**<br>\n';
    const message = 'Scrivi qui la tua nota.\n';
    insertNote(header, message);
});
document.getElementById('btn-tip').addEventListener('click', () => {
    const header = '\n>💡 **Suggerimento**<br>\n';
    const message = 'Scrivi qui il tuo suggerimento.\n';
    insertNote(header, message);
});
document.getElementById('btn-warning').addEventListener('click', () => {
    const header = '\n> ⚠️ **Avviso!**<br>\n';
    const message = 'Scrivi qui il tuo avviso.\n';
    insertNote(header, message);
});

// ::::::::::: KEYBOARD SHORTCUTS
// Could break browser's shortcut with focus on input area.

markdownInput.addEventListener('keydown', (e) => {
    // Check key Ctrl (Cmd on Mac) is pressed.
    const isModifier = e.ctrlKey || e.metaKey;

    if (isModifier) {
        switch (e.key.toLowerCase()) {
            // Bold
            case 'b':  
                e.preventDefault(); // Stop browser behaviour
                insertMarkdown('**', '**', '');
                break;
            // Italic   
            case 'i':
                e.preventDefault();
                insertMarkdown('*', '*', '');
                break;
            // Fixed-width text  
            case '`': 
                e.preventDefault();
                insertMarkdown('`', '`', '');
                break;
        }
    }
});

// :::::::::::::::::::::: VIEWS (Split, Input, Preview)

const editorWrapper = document.getElementById('editor-wrapper');
const btnSplit = document.getElementById('btn-view-split');
const btnInput = document.getElementById('btn-view-input');
const btnPreview = document.getElementById('btn-view-preview');

function changeView(viewClass, activeButton) {
    // Clean up every classes and add the selected class.
    editorWrapper.classList.remove('split-view', 'only-input', 'only-preview');
    editorWrapper.classList.add(viewClass);
    
    [btnSplit, btnInput, btnPreview].forEach(btn => btn.classList.remove('active'));
    activeButton.classList.add('active');
    
    // It forces "focus out" from input.
    if (viewClass === 'only-preview') {
        markdownInput.blur();
    } else {
        markdownInput.focus();
        markdownInput.setSelectionRange(0, 0);
        markdownInput.scrollTop = 0;
        window.scrollTo(0, 0);
    }
}

btnSplit.addEventListener('click', () => changeView('split-view', btnSplit));
btnInput.addEventListener('click', () => changeView('only-input', btnInput));
btnPreview.addEventListener('click', () => changeView('only-preview', btnPreview));

// :::::::::::::::::::::: SAVE

function getPagePath() {
    const form = document.getElementById("page-editor-form");
    const pathInput = document.getElementById("page-path");

    if (pathInput) {
        return pathInput.value.trim();
    }

    return form ? form.dataset.pagePath : "";
}

async function savePage() {
    const pagePath = getPagePath();

    if (!pagePath) {
        alert("Inserisci il percorso della pagina.");
        return;
    }

    const payload = {
        path: pagePath,
        markdown: markdownInput.value,
        html: DOMPurify.sanitize(preview.innerHTML)
    };

    const form = document.getElementById("page-editor-form");
    const isEdit = form.dataset.mode === "edit";

    const url = isEdit
        ? `/~modifica/${encodeURIComponent(pagePath)}`
        : "/~nuova";

    try {
        const response = await fetch(url, {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify(payload)
        });

        const result = await response.json();

        if (response.ok) {
            window.location.href = result.redirect_url;
        } else {
            alert(result.message || "Errore durante il salvataggio.");
        }
    } catch (error) {
        console.error(error);
        alert("Impossibile contattare il server.");
    }
}

document
    .getElementById("btn-save")
    .addEventListener("click", savePage);
