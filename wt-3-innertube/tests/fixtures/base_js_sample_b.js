// Sample YouTube player JavaScript (synthetic, derived from real shape).
// Captured: 2025-06. A second fixture in a different obfuscation style:
// standalone `function name()` declarations (rather than an object
// literal), used to make sure the cipher extractor handles both shapes.

function Ky(a, b) {
  var c = a[0];
  a[0] = a[b % a.length];
  a[b % a.length] = c;
}

function Jt(a, b) {
  a.splice(0, b);
}

function Vs(a) {
  a.reverse();
}

// Standalone decipher dispatcher.
function vfl_decipher(a) {
  a = a.split("");
  Jt(a, 2);
  Ky(a, 9);
  Vs(a);
  Ky(a, 4);
  return a.join("");
}

// N-sig function, standalone form.
function nsi(a) {
  var b = a.split("");
  var c = b.length;
  var d = b[0].charCodeAt(0);
  b[0] = String.fromCharCode(d ^ 1);
  return b.join("");
}

// Wrapper for the n-sig extractor.
var w = function () { return (a.get("n")) && (b = nsi(a)) };
