import { common } from '@kit.AbilityKit';
import {
  addTransform, compileProject, newProject, readCode, writeCode
} from '../services/BuilderDomain';
import { ProjectStore } from '../services/ProjectStore';

@Entry
@Component
struct BuilderPage {
  @State tenant: string = 'local';
  @State projectId: string = 'my_agent';
  @State nextNodeId: string = 'transform_1';
  @State codeView: boolean = false;
  @State cloudMode: boolean = false;
  @State source: string = writeCode(newProject('local', 'my_agent'), 'local');
  @State status: string = 'Projekt lokalny. Nie uruchomiono żadnych narzędzi.';

  private store(): ProjectStore {
    return new ProjectStore(getContext(this) as common.UIAbilityContext);
  }

  private localOnly(): void {
    if (this.cloudMode) throw new Error('Cloud zablokowany: brak uwierzytelnionego gateway HTTPS');
  }

  private updateFromNoCode(): void {
    try {
      this.localOnly();
      const project = addTransform(readCode(this.source, this.tenant), this.tenant, this.nextNodeId);
      this.source = writeCode(project, this.tenant);
      this.status = 'Dodano transformację ' + this.nextNodeId + ' (bez wykonania efektów).';
    } catch (error) {
      this.status = 'ODMOWA: ' + String(error);
    }
  }

  private validate(): void {
    try {
      const project = readCode(this.source, this.tenant);
      const plan = compileProject(project, this.tenant);
      this.status = 'Plan statyczny: ' + plan.ordered_nodes.join(' → ') + '. Efekty: nie wykonywano.';
    } catch (error) {
      this.status = 'ODMOWA: ' + String(error);
    }
  }

  private create(): void {
    try {
      this.localOnly();
      this.source = writeCode(newProject(this.tenant, this.projectId), this.tenant);
      this.status = 'Nowy projekt w pamięci. Zapis wymaga jawnego działania.';
    } catch (error) {
      this.status = 'ODMOWA: ' + String(error);
    }
  }

  private async save(): Promise<void> {
    try {
      this.localOnly();
      const project = readCode(this.source, this.tenant);
      if (project.project_id !== this.projectId) throw new Error('ID projektu nie zgadza się z formularzem');
      await this.store().save(project, this.tenant);
      this.status = 'Zapisano projekt lokalnie; nie opublikowano ani nie wykonano narzędzi.';
    } catch (error) {
      this.status = 'ODMOWA ZAPISU: ' + String(error);
    }
  }

  private load(): void {
    try {
      this.localOnly();
      const project = this.store().load(this.tenant, this.projectId);
      if (project === null) throw new Error('Projekt nie istnieje');
      this.source = writeCode(project, this.tenant);
      this.status = 'Odczytano projekt lokalny.';
    } catch (error) {
      this.status = 'ODMOWA ODCZYTU: ' + String(error);
    }
  }

  build() {
    Scroll() {
      Column({ space: 12 }) {
        Text('OCAF Agents Builder')
          .fontSize(28)
          .fontWeight(FontWeight.Bold)
          .fontColor('#D4AF37')

        Text('HarmonyOS NEXT • Code / NoCode • Local / Cloud')
          .fontSize(13)
          .fontColor('#9499A3')

        Row({ space: 10 }) {
          Button(this.codeView ? 'Code ✓' : 'Code')
            .onClick(() => { this.codeView = true; })
          Button(this.codeView ? 'NoCode' : 'NoCode ✓')
            .onClick(() => { this.codeView = false; })
        }

        Row({ space: 10 }) {
          Button(this.cloudMode ? 'Local' : 'Local ✓')
            .onClick(() => { this.cloudMode = false; this.status = 'Tryb lokalny.'; })
          Button(this.cloudMode ? 'Cloud ✓' : 'Cloud')
            .onClick(() => {
              this.cloudMode = true;
              this.status = 'Cloud: wymagane uwierzytelnienie i zaufany gateway. Operacje są zablokowane.';
            })
        }

        TextInput({ placeholder: 'Tenant (lokalny namespace)', text: this.tenant })
          .onChange((value: string) => { this.tenant = value; })
        TextInput({ placeholder: 'ID projektu', text: this.projectId })
          .onChange((value: string) => { this.projectId = value; })

        if (this.codeView) {
          Text('Code: edytuj kanoniczny JSON DAG')
            .fontColor('#F4F1EA')
          TextArea({ text: this.source, placeholder: 'JSON DAG' })
            .height(320)
            .onChange((value: string) => { this.source = value; })
        } else {
          Text('NoCode: typowany graf i bezpieczna transformacja')
            .fontColor('#F4F1EA')
          TextInput({ placeholder: 'ID nowego Transform', text: this.nextNodeId })
            .onChange((value: string) => { this.nextNodeId = value; })
          Button('Dodaj Transform przed wyjściem')
            .onClick(() => { this.updateFromNoCode(); })
          Text('Weryfikacja DAG: kliknij „Sprawdź plan”.')
            .fontColor('#9499A3')
          Text(this.source)
            .fontSize(11)
            .fontColor('#9499A3')
        }

        Row({ space: 8 }) {
          Button('Nowy').onClick(() => { this.create(); })
          Button('Sprawdź plan').onClick(() => { this.validate(); })
        }
        Row({ space: 8 }) {
          Button('Zapisz lokalnie').onClick(() => { this.save(); })
          Button('Otwórz').onClick(() => { this.load(); })
        }

        Text(this.status)
          .fontSize(13)
          .fontColor('#D4AF37')

        Text('Brak uruchamiania narzędzi, podpisywania, wdrożeń i płatnych operacji.')
          .fontSize(12)
          .fontColor('#9499A3')
      }
      .width('100%')
      .padding(20)
      .backgroundColor('#07080A')
    }
    .backgroundColor('#07080A')
    .width('100%')
    .height('100%')
  }
}
