export const QR_CODE = {
  EXPIRY_SECONDS: 180,
  EXPIRY_MS: 180 * 1000,
  SIZE: 'w-64 h-64',
} as const;

export const TIMING = {
  QR_EXPIRY_MS: 180 * 1000,
  REDIRECT_DELAY_MS: 2000,
} as const;

export const THEME = {
  GRADIENT_BG: 'bg-gradient-to-br from-blue-50 to-indigo-100',
  CARD_BG: 'bg-white rounded-lg shadow-lg',
} as const;

export const BUTTON = {
  PRIMARY: 'bg-blue-600 hover:bg-blue-700 text-white font-semibold py-3 px-4 rounded-lg transition-colors',
  SECONDARY: 'px-4 py-2 bg-white text-gray-700 rounded-lg hover:bg-gray-100 transition-colors border border-gray-300',
  SUCCESS: 'bg-green-600 hover:bg-green-700 text-white font-semibold py-3 px-4 rounded-lg transition-colors',
  WARNING: 'bg-orange-600 hover:bg-orange-700 text-white font-semibold py-3 px-4 rounded-lg transition-colors',
  NAVIGATION: 'px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors',
  NAVIGATION_PRIMARY: 'px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center gap-2',
  DISABLED: 'disabled:bg-gray-400',
} as const;
