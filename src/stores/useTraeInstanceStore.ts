import * as traeInstanceService from '../services/traeInstanceService';
import { createInstanceStore } from './createInstanceStore';

export const useTraeInstanceStore = createInstanceStore(
  traeInstanceService,
  'agtools.trae.instances.cache',
);
