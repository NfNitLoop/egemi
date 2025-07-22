# eGemi

An egui web browser for Gemini text and the small web.

## Principles:

eGemi takes inspiration from some of the reasoning behind Project Gemini:

### One Document, One Request

When you visit a URL, eGemi will load only that URL.

### Lack of features is a feature

 * No cookies
 * No JavaScript
   * No popups or annoying UI
 * No automatic loading of images, fonts, videos. 
   * Ex: cross-origin images/fonts can be used to track you across multiple sites.
 * No automatic redirects
   * Makes MITM click trackers, or outdated URLs more obvious.
 * No CSS
   * Read text in your style. (TODO: implement configuring styles.)


## Features:

 * HTTP(S) 1.1/2/3 support (TODO)
 * gemini protocol s upport. (TODO)