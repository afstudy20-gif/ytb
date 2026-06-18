// Sample YouTube player JavaScript (synthetic, derived from real shape).
// Captured: 2025-01. Used by tests/cipher.rs to exercise the cipher
// extractor and n-sig extractor against a realistic obfuscation pattern.
// The function and variable names below are deliberately mangled the way
// the YouTube player does it.
//
// The functions here are kept inside the JS subset this crate's hand-rolled
// interpreter supports (no `for`/`while` loops, no try/catch) so the same
// fixture can be used both for regex extraction *and* end-to-end
// decipher/transform evaluation.

// === Decipher function and its helpers ===

var Xva={
  Opa:function(a,b){
    var c=a[0];a[0]=a[b%a.length];a[b%a.length]=c
  },
  qza:function(a,b){
    a.splice(0,b)
  },
  rca:function(a){
    a.reverse()
  }
};

// The decipher dispatcher. The regex extractor locates this by the
// `:function(a){a=a.split("")` shape.
Xva.DP=function(a){a=a.split("");Xva.qza(a,3);Xva.Opa(a,17);Xva.rca(a);Xva.Opa(a,2);return a.join("")};

// === N-sig function ===
// YouTube wraps the n-sig call in `&&(b=FunctionName(a))`. We locate the
// function name from that wrapper, then read its body.
var mwa=function(a){
  var b=a.split("");
  var c=b[0].charCodeAt(0);
  var d=b[b.length-1].charCodeAt(0);
  b[0]=String.fromCharCode(d);
  b[b.length-1]=String.fromCharCode(c+1);
  return b.join("")
};

// The wrapper that ties the n-sig function to the player's URL handling.
// Real player JS has something like:
//   ...a.get("n"))&&(b=mwa(a))&&(c="&n="+b)
// We reproduce the relevant slice here so the n-sig name extractor works.
var dummy = function(){ return (a.get("n"))&&(b=mwa(a)) };
