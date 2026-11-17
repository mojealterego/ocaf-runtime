import { UIAbility } from '@kit.AbilityKit';
import { window } from '@kit.ArkUI';

export default class EntryAbility extends UIAbility {
  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/BuilderPage', (err) => {
      if (err.code) {
        console.error('OCAF Builder page load failed, code=' + err.code);
      }
    });
  }
}
