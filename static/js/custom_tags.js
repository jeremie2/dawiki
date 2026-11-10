// ::::::::::::::::::::::::::::::::::::::
// ::::::::::::::::::::::: CUSTOM TAGS ::
// ::::::::::::::::::::::::::::::::::::::

// :::::::::::::: Constant templates

const CONTRIB_LINK = ' <a href="#contribuisci" class="btn-contrib">Clicca qui!</a>';

const ALERT_TEMPLATES = {
    error: (version) => `
        <div class="ubuntu-alert alert-error">
            🛑 <strong>Errore nel tag UBU:</strong> la versione di Ubuntu "<strong>${version}</strong>" non esiste. Per favore, verificare i dati inseriti.
        </div>`,
    
    active: (versionsList) => `
        <div class="ubuntu-alert alert-active">
            Guida testata con Ubuntu: <strong>${versionsList}</strong>
        </div>`,
    
    pro: (versionsString) => `
        <div class="ubuntu-alert alert-pro">
            ⚠️ <strong>Guida non verificata da molto tempo.</strong> Può contenere informazioni utili per versioni Ubuntu Pro: ${versionsString}. Vuoi contribuire ad aggiornarla?${CONTRIB_LINK}
        </div>`,
    
    obsolete: () => `
        <div class="ubuntu-alert alert-obsolete">
            🛑 <strong>Guida verificata con versioni obsolete di Ubuntu</strong>, potrebbe non essere più valida. Vuoi contribuire ad aggiornarla?${CONTRIB_LINK}
        </div>`,

    forumLink: (url) => `
        <div class="link-forum-box">
            Problemi nella pagina? Segnala <a href="${url}">qui</a>.
        </div>`,
        
    forumError: () => `
        <div class="ubuntu-alert alert-error">
            🛑 <strong>Errore nel tag FORUM:</strong> L'URL inserito non sembra essere un link valido al forum di Ubuntu.
        </div>`      
};

/// :::::::::::::::::::::: HELPER FUNCTION "IS TAG"

/**
* Checks whether a given text tag is located within protected areas
* (such as <pre> code blocks, <code> tags, or input/textarea elements). 
*
* @param {string} tagText - The full tag to check (e.g., "[UBU 24.04]" or "[IND]")
* @param {HTMLElement} container - The preview area of ​​your editor (e.g., previewArea)
* @returns {boolean} - True if the tag is inside a code block and should be ignored; otherwise, False. 
*/

function isTagInsideCode(tagText, container) {
    if (!container || !tagText) return false;

    // Selects every blocks and element with raw text inside.
    const protectedElements = container.querySelectorAll('code, pre, textarea, input');
    
    // Checks if text is part of a protectedElements.
    for (const element of protectedElements) {
        // For input/textarea it checks `.value`. For html tags checks `.textContent``
        const content = (element.tagName === 'INPUT' || element.tagName === 'TEXTAREA') 
            ? element.value 
            : element.textContent;

        if (content && content.includes(tagText)) {
            return true; // Protected tag! It mast remain raw.
        }
    }
    
    return false; // Safe tag, it can be rendered.
}

// ::::::::::::::::::::::::::::::::::::::
// :::::::::::::::::::::: TOC :::::::::::
// ::::::::::::::::::::::::::::::::::::::

// :::::::::::::::::::::: HELPER FUNCTION to build ordered lists

function generateTocOl(headings) {
  let tocHtml = '<ol>';
  let currentLevel = 1;

  headings.forEach((heading, index) => {
    const uniqueId = 'heading-paragraph-' + index;
    heading.id = uniqueId;

    // Extracts the level from the tag, e.g. "h2" -> 2.
    const level = parseInt(heading.tagName.substring(1));

    // Scende di livello: h1 -> h2 -> h3
    if (level > currentLevel) {
      while (level > currentLevel) {
        tocHtml += '<ol>';
        currentLevel++;
      }
    }
    // Moves up a level: h3 -> h2 -> h1.
    else if (level < currentLevel) {
      while (level < currentLevel) {
        tocHtml += '</ol></li>';
        currentLevel--;
      }
    }
    // On the same level as the previous title.
    else if (index > 0) {
      tocHtml += '</li>';
    }

    tocHtml += `<li><a href="#${uniqueId}">${heading.innerText}</a>`;
  });

  // Closes all open lists.
  while (currentLevel > 0) {
    tocHtml += '</ol>';

    if (currentLevel > 1) {
      tocHtml += '</li>';
    }

    currentLevel--;
  }

  return tocHtml;
}

// :::::::::::::::::::::: MAIN FUNCTION toc

function generateTableOfContents(previewArea) {
  const currentHtml = previewArea.innerHTML;
  let foundPlaceholder = "";
  let maxLevel = 3;

  const sidebarToc = document.getElementById('sidebar-toc');

  // Removes any remnants of the previous index.
  if (sidebarToc) {
    sidebarToc.innerHTML = "";
    sidebarToc.classList.remove('has-toc');
  }

  if (currentHtml.includes('[IND-3]')) {
    foundPlaceholder = '[IND-3]';
    maxLevel = 3;
  } else if (currentHtml.includes('[IND-2]')) {
    foundPlaceholder = '[IND-2]';
    maxLevel = 2;
  } else if (currentHtml.includes('[IND]')) {
    foundPlaceholder = '[IND]';
    maxLevel = 1;
  }

  if (!foundPlaceholder) return;

  if (isTagInsideCode(foundPlaceholder, previewArea)) return;

  const selectors = [];

  for (let i = 1; i <= maxLevel; i++) {
    selectors.push('h' + i);
  }

  const headings = previewArea.querySelectorAll(selectors.join(', '));

  if (headings.length === 0) return;

  const tocOl = generateTocOl(headings);

  const detailsTocHtml = `
    <details class="toc-container mobile-only-toc">
      <summary style="cursor:pointer; user-select:none;">
        <strong>Indice</strong>
      </summary>
      ${tocOl}
    </details>
  `;

  const freshHtml = previewArea.innerHTML;

  if (freshHtml.includes(`<p>${foundPlaceholder}</p>`)) {
    previewArea.innerHTML = freshHtml.replace(
      `<p>${foundPlaceholder}</p>`,
      detailsTocHtml
    );
  } else {
    previewArea.innerHTML = freshHtml.replace(
      foundPlaceholder,
      detailsTocHtml
    );
  }

  // Sidebar is populated and made visible only if an index exists.
  if (sidebarToc) {
    sidebarToc.innerHTML = `
      <div class="desktop-only-toc">
        ${tocOl}
      </div>
    `;

    sidebarToc.classList.add('has-toc');
  }

  const detailsEl = previewArea.querySelector('.toc-container');

  if (detailsEl) {
    detailsEl.querySelectorAll('a').forEach(link => {
      link.addEventListener('click', () => {
        detailsEl.open = false;
      });
    });
  }
}

// ::::::::::::::::::::::::::::::::::::::
// :::::::::::::::::::::: UBU VERSIONS ::
// ::::::::::::::::::::::::::::::::::::::

// :::::::::::::: helper functions

// Update set list of Ubuntu versions.
function generaSetVersioniUbuntu() {
    const releases = [
        '6.06',  '6.10',   '7.04',  '7.10',  '8.04',  '8.10',
        '9.04',  '9.10',  '12.04', '12.10', '13.04', '13.10',
        '14.04', '14.10', '15.04', '15.10', '10.04', '10.10',
        '11.04', '11.10', '16.04', '16.10', '17.04', '17.10',
        '18.04', '18.10', '19.04', '19.10', '20.04', '20.10',
        '21.04', '21.10', '22.04', '22.10', '23.04', '23.10',
        '24.04', '24.10', '25.04', '25.10', '26.04', '26.10'
    ];

    const currentYear = new Date().getFullYear();
    const lastTwoCorrectDigits = currentYear % 100;  // E.g.: 2026 -> 26
    // From last year element in `releases` it gets the year:
    // '25.10' -> 25 as int
    const lastElementYear = parseInt(releases.at(-1).split('.')[0], 10);
for ( let year = lastElementYear + 1; year <= lastTwoCorrectDigits; year++) {

        // `.04` always released in the first half of the year.
        if (year < lastTwoCorrectDigits || (year === lastTwoCorrectDigits && new Date().getMonth() >= 3)) {
            releases.push(`${year}.04`);
        }

        // `.10` it is added only if the year has passed or if
        // we are already in October or later (month >= 9 in JS)
        if (year < lastTwoCorrectDigits || (year === lastTwoCorrectDigits && new Date().getMonth() >= 9)) {
            releases.push(`${year}.10`);
        }
    }

    return new Set(releases);
}

function validateVersions(versionsArray, validVersionsSet) {
    let invalidVersion = null;

    for (const ver of versionsArray) {
        if (!ver) continue;
        // Se la versione non esiste nel Set ufficiale, cattura l'errore
        if (!validVersionsSet.has(ver)) {
            invalidVersion = ver;
            break;
        }
    }

    return {
        isValid: invalidVersion === null,
        invalidVersion: invalidVersion
    };
}

// ::::::::::::::::::::::::::::::::::::::::::::::::::::::::

function getHighestSupportStatus(versionsArray) {
    // 
    const now = new Date();
    const currentYear = now.getFullYear();
    const currentMonth = now.getMonth() + 1; // January is 0 -> +1
    const currentTotalMonths = (currentYear * 12) + currentMonth;

    // For LTS versions:
    // maxStatus = 2 -> standard 5 years support range.
    // maxStatus = 1 -> pro 10 years support range.
    // maxStatus = 0 -> obsolete. (Legacy Support is out of our goals)
    let maxStatus = 0;
    // Map with state of every specific version.

    // We take note of every status for ALERT_TEMPLATES:
    // .active() shows only standard support version
    // .pro() shows only pro support version
    const detailedStatuses = {};

    versionsArray.forEach(ver => {
        if (!ver) return;
        const parts = ver.split('.');
        if (parts.length !== 2) return;

        // Before reaching the year 2100 remember to modify this part! ;)
        const releaseYear = parseInt("20" + parts[0], 10);
        const releaseMonth = parseInt(parts[1], 10);

        // LTS versions on April in even-numbered years.
        const isLTS = (releaseMonth === 4 && releaseYear % 2 === 0);
        // LTS standard support: 5 years -> 60 months.
        // LTS Pro support: 10 years -> 120 months.
        // If NOT LTS it gets support for 9 months.
        const standardSupportDuration = isLTS ? (5 * 12) : 9;
        const proSupportDuration = isLTS ? (10 * 12) : 9;

        // Sort of age-score in months for a release.
        // E.g. 24.04  -> (2024 * 12) + 4 = 24.292 months
        const releaseTotalMonths = (releaseYear * 12) + releaseMonth;
        // 24.04 end of standars support after 24.292 + 60 = 24.352 months
        const endStandardMonths = releaseTotalMonths + standardSupportDuration;
        // 24.04 end of pro support after 24.292 + 120 = 24.412 months
        const endProMonths = releaseTotalMonths + proSupportDuration;
        
        let currentVerStatus = 0;

        // If months <= endStandardMonths → maxStatus = 2 (Active Standard Support).
        // If endStandardMonths <= months <=  endProMonths → maxStatus = 1 (Pro Support).
        // Else maxStatus = 0 (Obsolete).
        if (currentTotalMonths <= endStandardMonths) {currentVerStatus = 2;
            if (maxStatus < 2) maxStatus = 2;
        } else if (isLTS && currentTotalMonths <= endProMonths) {
            currentVerStatus = 1;
            if (maxStatus < 1) maxStatus = 1;
        }

        detailedStatuses[ver] = currentVerStatus;
    });

    return { maxStatus, detailedStatuses };
}

// :::::::::::::: main function

function parseUbuntuVersions(previewArea) {
  const currentHtml = previewArea.innerHTML;
  
  // Intercept tag [UBU XY.ZW AB.CD ...]
  const versionRegex = /\[UBU\s+([^\]]+)\]/gi;

  // `exec()` searches the given pattern (versionRegex)
  // into `currentHtml`. It returns `null` if it doesn't find it.
  // Otherwise it returns an array where:
  // match[0]: the whole tag like [UBU 24.04 24.10].
  // match[1]: text inside [UBU ] like "24.04 24.10".
  const match = versionRegex.exec(currentHtml);
  
  if (!match) return; // Exit if there is NO tag.
  
  const fullTag = match[0];        // E.g. "[UBU 24.04 26.10]"
  const versionsString = match[1]; // E.g. "24.04 26.10"
  
  // It makes UBU tag not active inside raw text tags (<code>, <pre>).
  if (isTagInsideCode(fullTag, previewArea)) return;

  // Separating each version and generating an array.
  const rawVersions = versionsString.trim().split(/\s+/);
  // No duplicates creating a set.
  const versions = [...new Set(rawVersions)];

  const validUbuntuVersions = generaSetVersioniUbuntu();
  const validation = validateVersions(versions, validUbuntuVersions);
  
  let alertBoxHtml = '';

  if (!validation.isValid) {
      // If not valid -> error template.
      alertBoxHtml = ALERT_TEMPLATES.error(validation.invalidVersion);
  } else {
      // Which kind of support (standard/pro).
      const { maxStatus, detailedStatuses } = getHighestSupportStatus(versions);
      
      // Which kind of template based on kind of support.
      if (maxStatus === 2) {
          // Only active version will be shown.
          const activeVersions = versions
            .filter(ver => detailedStatuses[ver] === 2)
            .sort();
          alertBoxHtml = ALERT_TEMPLATES.active(activeVersions.join(' '));
      } else if (maxStatus === 1) {
          // Only pro version will be shown.
          const proVersions = versions
             .filter(ver => detailedStatuses[ver] === 1)
             .sort();
          alertBoxHtml = ALERT_TEMPLATES.pro(proVersions.join(', '));
      } else {
          alertBoxHtml = ALERT_TEMPLATES.obsolete();
      }
  }

  const freshHtml = previewArea.innerHTML;
  if (freshHtml.includes(`<p>${fullTag}</p>`)) {
      previewArea.innerHTML = freshHtml.replace(`<p>${fullTag}</p>`, alertBoxHtml);
  } else {
      previewArea.innerHTML = freshHtml.replace(fullTag, alertBoxHtml);
  }
}

// ::::::::::::::::::::::::::::::::::::::
// ::::::::::::::::::::: LINK TO FORUM ::
// ::::::::::::::::::::::::::::::::::::::

function parseForumTag(previewArea) {
  const currentHtml = previewArea.innerHTML;
  // Intercepts tag FORUM.
  const forumRegex = /\[FORUM\s+([^\]]+)\]/gi;
  const match = forumRegex.exec(currentHtml);
  
  if (!match) return; // Exit if there's no tag.
  
  const fullTag = match[0];
  let insideContent = match[1].trim();
  
  if (isTagInsideCode(fullTag, previewArea)) return;
  
  if (insideContent.includes('<a ')) {
    const tempDiv = document.createElement('div');
    tempDiv.innerHTML = insideContent;
    const anchor = tempDiv.querySelector('a');
    if (anchor) {
      insideContent = anchor.getAttribute('href').trim();
    }
  }
  
  let alertBoxHtml = '';
  
  // Link validation.
  if (insideContent.startsWith('http://') || insideContent.startsWith('https://') || insideContent.startsWith('www.')) {
    const finalUrl = insideContent.startsWith('www.') ? `https://${insideContent}` : insideContent;
    alertBoxHtml = ALERT_TEMPLATES.forumLink(finalUrl);
  } else {
    alertBoxHtml = ALERT_TEMPLATES.forumError();
  }
  
  // Sobstitution into preview.
  const freshHtml = previewArea.innerHTML;
  if (freshHtml.includes(`<p>${fullTag}</p>`)) {
    previewArea.innerHTML = freshHtml.replace(`<p>${fullTag}</p>`, alertBoxHtml);
  } else {
    previewArea.innerHTML = freshHtml.replace(fullTag, alertBoxHtml);
  }
}
