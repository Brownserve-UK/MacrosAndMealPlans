import { createTheme, type ThemeOptions } from '@mui/material/styles';

const SANS = '"Inter", system-ui, -apple-system, "Segoe UI", Roboto, sans-serif';
const DISPLAY = '"Fraunces", "Iowan Old Style", Georgia, serif';

const radius = 14;

const options: ThemeOptions = {
  cssVariables: { colorSchemeSelector: 'data-mui-color-scheme' },
  colorSchemes: {
    light: {
      palette: {
        primary: {
          main: '#2E6B4F',
          light: '#4C8B6C',
          dark: '#1D4834',
          contrastText: '#FFFFFF',
        },
        secondary: { main: '#8A6244' },
        success: { main: '#2E6B4F' },
        warning: { main: '#A4682A' },
        background: { default: '#FAF8F5', paper: '#FFFFFF' },
        text: { primary: '#1C1A17', secondary: '#6E675D', disabled: '#A39B8E' },
        divider: 'rgba(28, 26, 23, 0.10)',
        action: { hover: 'rgba(46, 107, 79, 0.06)', selected: 'rgba(46, 107, 79, 0.10)' },
      },
    },
    dark: {
      palette: {
        primary: {
          main: '#86C3A0',
          light: '#A7D7BB',
          dark: '#4C8B6C',
          contrastText: '#0E1A13',
        },
        secondary: { main: '#D0A57F' },
        success: { main: '#86C3A0' },
        warning: { main: '#D9A063' },
        background: { default: '#141310', paper: '#1C1A16' },
        text: { primary: '#F1EDE5', secondary: '#A69E90', disabled: '#6E675D' },
        divider: 'rgba(241, 237, 229, 0.12)',
        action: { hover: 'rgba(134, 195, 160, 0.08)', selected: 'rgba(134, 195, 160, 0.14)' },
      },
    },
  },
  shape: { borderRadius: radius },
  spacing: 8,
  typography: {
    fontFamily: SANS,
    h1: {
      fontFamily: DISPLAY,
      fontSize: '2.125rem',
      fontWeight: 600,
      letterSpacing: '-0.015em',
      lineHeight: 1.15,
    },
    h2: {
      fontFamily: DISPLAY,
      fontSize: '1.5rem',
      fontWeight: 600,
      letterSpacing: '-0.01em',
    },
    h3: { fontSize: '1rem', fontWeight: 600, letterSpacing: '0.01em' },
    subtitle1: { fontSize: '1rem', fontWeight: 500 },
    subtitle2: { fontSize: '0.875rem', fontWeight: 600 },
    body2: { fontSize: '0.875rem', lineHeight: 1.6 },
    caption: { fontSize: '0.78rem', lineHeight: 1.5 },
    button: { textTransform: 'none', fontWeight: 600, letterSpacing: 0 },
  },
  components: {
    MuiCssBaseline: {
      styleOverrides: (theme) => ({
        body: { WebkitFontSmoothing: 'antialiased' },
        '.app-link': {
          color: (theme.vars ?? theme).palette.primary.main,
          textDecoration: 'none',
          fontWeight: 500,
          '&:hover': { textDecoration: 'underline' },
        },
        '.numeral': { fontVariantNumeric: 'tabular-nums' },
      }),
    },
    MuiPaper: {
      defaultProps: { elevation: 0 },
      styleOverrides: {
        root: ({ theme }) => ({
          backgroundImage: 'none',
          border: `1px solid ${(theme.vars ?? theme).palette.divider}`,
        }),
      },
    },
    MuiButton: {
      defaultProps: { disableElevation: true },
      styleOverrides: {
        root: { borderRadius: 10, paddingInline: 18, paddingBlock: 9 },
        sizeSmall: { paddingInline: 12, paddingBlock: 5 },
      },
    },
    MuiOutlinedInput: {
      styleOverrides: { root: { borderRadius: 10 } },
    },
    MuiTextField: { defaultProps: { size: 'small' } },
    MuiChip: {
      styleOverrides: {
        root: { fontWeight: 500, borderRadius: 8 },
        sizeSmall: { height: 24 },
      },
    },
    MuiAppBar: {
      defaultProps: { elevation: 0, color: 'inherit' },
      styleOverrides: {
        root: ({ theme }) => ({
          borderBottom: `1px solid ${(theme.vars ?? theme).palette.divider}`,
          backgroundColor: (theme.vars ?? theme).palette.background.default,
          backgroundImage: 'none',
        }),
      },
    },
    MuiDrawer: {
      styleOverrides: {
        paper: ({ theme }) => ({
          border: 'none',
          borderRight: `1px solid ${(theme.vars ?? theme).palette.divider}`,
          backgroundColor: (theme.vars ?? theme).palette.background.default,
        }),
      },
    },
    MuiListItemButton: {
      styleOverrides: { root: { borderRadius: 10 } },
    },
    MuiTooltip: {
      defaultProps: { arrow: true },
      styleOverrides: { tooltip: { fontSize: '0.78rem', borderRadius: 8, padding: '6px 10px' } },
    },
    MuiDialog: {
      styleOverrides: { paper: { borderRadius: radius } },
    },
    MuiAlert: {
      styleOverrides: { root: { borderRadius: 12 } },
    },
  },
};

export const theme = createTheme(options);
