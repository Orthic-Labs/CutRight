#!/usr/bin/env node

// CR-V2-B1-008 PROVENANCE HEADER — CutRight v2 adaptation.
// Upstream: workspace-capabilities @ 6ee21f03a787e7b57dc412760a8996ea7a235302,
// tools/skills/designer/engine (see skills/designer/THIRD_PARTY.yml).
// This script is INERT PROVENANCE in the base CutRight runtime: it is never
// invoked as a bare shell command and performs no workspace mutation by itself.
// Execution happens only as the typed capability cutright://capability/designer.detect_antipatterns
// inside the signed runtime pack, emitting AssetDelivery records (no direct mutation).
// See skills/designer/CUTRIGHT-ADAPTATION.md for the capability mapping.

/**
 * Anti-Pattern Detector for Impeccable
 * Copyright (c) 2026 Paul Bakaus
 * SPDX-License-Identifier: Apache-2.0
 *
 * Public API facade. Runtime engines live under cli/engine/engines/.
 */

import { detectCli } from './cli/main.mjs';

export { ANTIPATTERNS, RULE_ENGINE_SUPPORT, getAntipattern, getRulesForCategory, getRuleEngineSupport } from './registry/antipatterns.mjs';
export { SAFE_TAGS, BORDER_SAFE_TAGS, OVERUSED_FONTS, GENERIC_FONTS, KNOWN_SERIF_FONTS } from './shared/constants.mjs';
export { isNeutralColor, parseRgb, relativeLuminance, contrastRatio, parseGradientColors, hasChroma, getHue, colorToHex } from './shared/color.mjs';
export { isFullPage } from './shared/page.mjs';
export {
  checkElementBorders,
  checkElementMotion,
  checkElementGlow,
  checkPageTypography,
  checkPageLayout,
  checkHtmlPatterns,
} from './rules/checks.mjs';
export { createDetectorProfile, summarizeDetectorProfile } from './profile/profiler.mjs';
export {
  parseFrontmatter as parseDesignFrontmatter,
  normalizeDesignSystem,
  loadDesignSystemForCwd,
  checkSourceDesignSystem,
  collectStaticDesignSystemFindings,
} from './design-system.mjs';
export { detectHtml } from './engines/static-html/detect-html.mjs';
export { detectUrl, createBrowserDetector } from './engines/browser/detect-url.mjs';
export { detectText, extractStyleBlocks, extractCSSinJS } from './engines/regex/detect-text.mjs';
export {
  walkDir,
  SCANNABLE_EXTENSIONS,
  SKIP_DIRS,
  buildImportGraph,
  resolveImport,
  detectFrameworkConfig,
  isPortListening,
  FRAMEWORK_CONFIGS,
} from './node/file-system.mjs';
export { formatFindings, detectCli } from './cli/main.mjs';

const isMainModule = process.argv[1]?.endsWith('detect-antipatterns.mjs') ||
  process.argv[1]?.endsWith('detect-antipatterns.mjs/');
if (isMainModule) detectCli();
