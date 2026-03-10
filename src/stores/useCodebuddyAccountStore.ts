import {
  CodebuddyAccount,
  getCodebuddyAccountDisplayEmail,
  getCodebuddyPlanBadge,
  getCodebuddyUsage,
} from '../types/codebuddy';
import * as codebuddyService from '../services/codebuddyService';
import { createProviderAccountStore } from './createProviderAccountStore';

const CODEBUDDY_ACCOUNTS_CACHE_KEY = 'agtools.codebuddy.accounts.cache';

export const useCodebuddyAccountStore = createProviderAccountStore<CodebuddyAccount>(
  CODEBUDDY_ACCOUNTS_CACHE_KEY,
  {
    listAccounts: codebuddyService.listCodebuddyAccounts,
    deleteAccount: codebuddyService.deleteCodebuddyAccount,
    deleteAccounts: codebuddyService.deleteCodebuddyAccounts,
    injectAccount: codebuddyService.injectCodebuddyToVSCode,
    refreshToken: codebuddyService.refreshCodebuddyToken,
    refreshAllTokens: codebuddyService.refreshAllCodebuddyTokens,
    importFromJson: codebuddyService.importCodebuddyFromJson,
    exportAccounts: codebuddyService.exportCodebuddyAccounts,
    updateAccountTags: codebuddyService.updateCodebuddyAccountTags,
  },
  {
    getDisplayEmail: getCodebuddyAccountDisplayEmail,
    getPlanBadge: getCodebuddyPlanBadge,
    getUsage: getCodebuddyUsage,
  },
);
