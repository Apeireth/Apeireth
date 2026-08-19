import createDOMPurify from 'dompurify';
import hljs from 'highlight.js/lib/common';
import katex from 'katex';
import {marked, Renderer} from 'marked';

const MATH_MARKER = '@@PATTERN_MATH_';

const renderer = new Renderer();

renderer.code = ({text, lang}) => {
  const language = (lang || '').trim().split(/\s+/)[0]?.toLowerCase() || '';
  const highlighted = language && hljs.getLanguage(language)
    ? hljs.highlight(text, {language}).value
    : text.trim() ? hljs.highlightAuto(text).value : '';
  const languageClass = language ? ` class="language-${escapeAttribute(language)}"` : '';
  return `<pre class="md-code"><code${languageClass}>${highlighted}</code></pre>\n`;
};

renderer.codespan = ({text}) => `<code class="md-inline">${escapeHtml(text)}</code>`;
renderer.html = ({text}) => escapeHtml(text);

/**
 * Render assistant Markdown with GFM, safe links/images, syntax-highlighted
 * fenced code, and KaTeX math. Math is protected before Markdown parsing so
 * backslashes are not treated as Markdown escapes.
 */
export function renderMarkdown(source: string): string {
  const text = String(source || '').replace(/\r\n/g, '\n');
  if (!text) return '';

  const formulas: Array<{expression: string; displayMode: boolean}> = [];
  const protectedText = protectMath(text, formulas);
  const parsed = marked.parse(protectedText, {
    gfm: true,
    breaks: true,
    renderer,
  }) as string;
  const withMath = parsed.replace(/@@PATTERN_MATH_(\d+)@@/g, (_match, index: string) => {
    const formula = formulas[Number(index)];
    if (!formula) return '';
    try {
      return katex.renderToString(formula.expression, {
        displayMode: formula.displayMode,
        throwOnError: false,
        strict: 'ignore',
        output: 'htmlAndMathml',
      });
    } catch {
      return `<code class="md-inline md-math-error">${escapeHtml(formula.expression)}</code>`;
    }
  });

  // Keep the existing chat typography classes while letting marked own the
  // parsing. DOMPurify removes raw HTML/script URLs from model output.
  const styled = withMath
    .replace(/<p>/g, '<p class="md-p">')
    .replace(/<h([1-6])>/g, '<h$1 class="md-h">')
    .replace(/<ul(?: class="[^"]*")?>/g, '<ul class="md-list">')
    .replace(/<ol(?: class="[^"]*")?>/g, '<ol class="md-list">')
    .replace(/<blockquote>/g, '<blockquote class="md-quote">')
    .replace(/<table>/g, '<table class="md-table">')
    .replace(/<hr>/g, '<hr class="md-hr"/>')
    .replace(/<img /g, '<img class="md-img" ')
    .replace(/<ul class="md-list">(?=[\s\S]*?<input)/g, '<ul class="md-list md-task">');
  const purifier = typeof window !== 'undefined' ? createDOMPurify(window) : null;
  return purifier
    ? purifier.sanitize(styled, {ADD_ATTR: ['target', 'rel']})
    : styled;
}

function protectMath(source: string, formulas: Array<{expression: string; displayMode: boolean}>): string {
  // Fenced code is opaque to math replacement. This prevents `$HOME` or
  // LaTeX-looking strings inside code samples from being rendered as math.
  const parts = source.split(/(```[\s\S]*?```|~~~[\s\S]*?~~~)/g);
  return parts.map((part, index) => {
    if (index % 2 === 1) return part;

    const inlineCode: string[] = [];
    let segment = part.replace(/`[^`\n]+`/g, (code) => {
      const marker = `@@PATTERN_INLINE_${inlineCode.length}@@`;
      inlineCode.push(code);
      return marker;
    });
    segment = segment.replace(/\\\[([\s\S]*?)\\\]|\\\(([\s\S]*?)\\\)|\$\$([\s\S]*?)\$\$|(?<!\$)\$([^$\n]+?)\$(?!\$)/g, (_match, displayBracket, inlineBracket, displayDollar, inlineDollar) => {
      const expression = String(displayBracket ?? inlineBracket ?? displayDollar ?? inlineDollar ?? '').trim();
      const displayMode = displayBracket !== undefined || displayDollar !== undefined;
      const marker = `${MATH_MARKER}${formulas.length}@@`;
      formulas.push({expression, displayMode});
      return marker;
    });
    return segment.replace(/@@PATTERN_INLINE_(\d+)@@/g, (_match, item: string) => inlineCode[Number(item)] || '');
  }).join('');
}

function escapeHtml(text: string): string {
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function escapeAttribute(text: string): string {
  return escapeHtml(text).replace(/`/g, '&#96;');
}
