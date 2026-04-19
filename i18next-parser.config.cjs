// i18next-parser.config.cjs
// Config for the Phase 2 i18n lint gate.
// AC: `npx i18next-parser --dry-run --fail-on-untranslated-strings` exits 0.
//
// Note: i18next-parser 9.x is deprecated (successor is i18next-cli) but is
// used here per the plan AC. Phase 9 audit can evaluate migration.

/** @type {import('i18next-parser').UserConfig} */
module.exports = {
  // Source files that contain t('key') calls.
  input: ['src/**/*.{ts,tsx}'],

  // Output locale files. The $LOCALE placeholder expands to each locale.
  output: 'src/locales/$LOCALE.json',

  // Supported locales — English only in v1.
  locales: ['en'],

  // Default namespace (matches i18next init in src/main.tsx).
  defaultNamespace: 'translation',

  // Function names to look for. `t` is the react-i18next hook result.
  functions: ['t'],

  // Separator for nested keys, e.g. 'panels.sidePanel' -> panels -> sidePanel.
  namespaceSeparator: false,
  keySeparator: '.',

  // Do NOT add missing keys to the output file during dry-run or live runs —
  // all keys must be hand-managed in src/locales/en.json per the plan contract.
  skipDefaultValues: true,

  // Keep existing translations that are not found in source (dynamic keys
  // like `panels.${activeActivity}` are used at runtime but not detectable
  // by static analysis — mark them with /* i18next-parser-keep */ comments
  // or via the keepRemoved option so they aren't stripped).
  keepRemoved: true,

  // Verbose output so CI shows which keys were found.
  verbose: false,

  // i18next-parser's `--fail-on-untranslated-strings` flag: fail if any key
  // found in source is missing from the locale output file.
  // All currently-used keys exist in src/locales/en.json — gate should pass.
};
