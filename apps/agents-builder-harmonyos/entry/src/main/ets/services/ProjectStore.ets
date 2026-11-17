import { preferences } from '@kit.ArkData';
import { common } from '@kit.AbilityKit';
import { BuilderProject, readCode, writeCode } from './BuilderDomain';

// Local-only preferences. Tenant is a namespace, NOT an authenticated identity.
export class ProjectStore {
  private readonly database: preferences.Preferences;

  constructor(context: common.UIAbilityContext) {
    this.database = preferences.getPreferencesSync(context, { name: 'ocaf_builder_local_v1' });
  }

  private key(tenant: string, projectId: string): string {
    if (!/^[A-Za-z0-9_.-]{1,64}$/.test(tenant) ||
        !/^[A-Za-z0-9_.-]{1,64}$/.test(projectId) ||
        tenant === '..' || projectId === '..' || tenant === '.' || projectId === '.') {
      throw new Error('Invalid local namespace');
    }
    return 'dag:' + tenant + ':' + projectId;
  }

  load(tenant: string, projectId: string): BuilderProject | null {
    const value = this.database.getSync(this.key(tenant, projectId), '');
    if (typeof value !== 'string' || value.length === 0) return null;
    return readCode(value, tenant);
  }

  async save(project: BuilderProject, tenant: string): Promise<void> {
    const serialized = writeCode(project, tenant);
    this.database.putSync(this.key(tenant, project.project_id), serialized);
    // Await durable flush; callers only report success once it completes.
    await this.database.flush();
  }
}
