import { common } from '@kit.AbilityKit';
import {
  BuilderProject, addTransform, addTool, removeStep, renameNode, updateTool,
  compileProject, newProject, readCode, writeCode
} from '../services/BuilderDomain';
import { ProjectStore } from '../services/ProjectStore';

@Entry
@Component
struct BuilderPage {
  @State tenant: string = 'local';
  @State projectId: string = 'my_agent';
  @State nextNodeId: string = 'transform_1';
  @State renameTo: string = 'renamed';
  @State toolName: string = 'example.tool';
  @State toolCost: string = '1';
  @State saving: boolean = false;
  @State codeView: boolean = false;
  @State cloudMode: boolean = false;
  @State source: string = writeCode(newProject('local', 'my_agent'), 'local');
  @State exchange: string = '';
  @State status: string = 'Projekt lokalny. Nie uruchomiono żadnych narzędzi.';

  private store(): ProjectStore {
    return new ProjectStore(getContext(this) as common.UIAbilityContext);
  }

  private localOnly(): void {
    if (this.saving) throw new Error('Zapis w toku');
    if (this.cloudMode) throw new Error('Cloud zablokowany: brak uwierzytelnionego gateway HTTPS');
  }

  private checkedProject(source: string): BuilderProject {
    const project = readCode(source, this.tenant);
    if (project.project_id !== this.projectId) throw new Error('ID projektu nie zgadza się z formularzem');
    return project;
  }

  private exportProject(): void {
    try {
      this.localOnly();
      this.exchange = writeCode(this.checkedProject(this.source), this.tenant);
      this.status = 'Eksport JSON do pola wymiany. Nie podpisano ani nie opublikowano projektu.';
    } catch (error) {
      this.exchange = '';
      this.status = 'ODMOWA EKSPORTU: ' + String(error);
    }
  }

  private importProject(): void {
    try {
      this.localOnly();
      const canonical = writeCode(this.checkedProject(this.exchange), this.tenant);
      this.source = canonical;
      this.status = 'Import JSON do pamięci. Zapis wymaga jawnego działania.';
    } catch (error) {
      this.status = 'ODMOWA IMPORTU: ' + String(error);
    }
  }

  private updateFromNoCode(operation: string): void {
    try {
      this.localOnly();
      const before = this.checkedProject(this.source);
      let project: BuilderProject;
      if (operation === 'transform') {
        project = addTransform(before, this.tenant, this.nextNodeId);
      } else if (operation === 'remove') {
        project = removeStep(before, this.tenant, this.nextNodeId);
      } else if (operation === 'rename') {
        project = renameNode(before, this.tenant, this.nextNodeId, this.renameTo);
      } else if (operation === 'tool' || operation === 'update-tool') {
        if (!/^[1-9][0-9]{0,12}$/.test(this.toolCost)) throw new Error('Nieprawidłowy koszt');
        const cost = Number(this.toolCost);
        project = operation === 'tool'
          ? addTool(before, this.tenant, this.nextNodeId, this.toolName, cost)
          : updateTool(before, this.tenant, this.nextNodeId, this.toolName, cost);
      } else {
        throw new Error('Nieobsługiwana operacja');
      }
      this.source = writeCode(project, this.tenant);
      this.status = 'Zmieniono graf r' + project.revision + ' — bez wykonywania efektów.';
    } catch (error) {
      this.status = 'ODMOWA: ' + String(error);
    }
  }

  private validate(): void {
    try {
      this.localOnly();
      const project = this.checkedProject(this.source);
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
    if (this.saving) return;
    try {
      this.localOnly();
      const project = this.checkedProject(this.source);
      this.saving = true;
      await this.store().save(project, this.tenant);
      this.status = 'Zapisano projekt lokalnie; nie opublikowano ani nie wykonano narzędzi.';
    } catch (error) {
      this.status = 'ODMOWA ZAPISU: ' + String(error);
    } finally {
      this.saving = false;
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
            .enabled(!this.saving)
            .onClick(() => { this.cloudMode = false; this.exchange = ''; this.status = 'Tryb lokalny.'; })
          Button(this.cloudMode ? 'Cloud ✓' : 'Cloud')
            .enabled(!this.saving)
            .onClick(() => {
              this.cloudMode = true;
              this.exchange = '';
              this.status = 'Cloud: wymagane uwierzytelnienie i zaufany gateway. Operacje są zablokowane.';
            })
        }

        TextInput({ placeholder: 'Tenant (lokalny namespace)', text: this.tenant })
          .enabled(!this.saving && !this.cloudMode)
          .onChange((value: string) => { if (!this.saving && !this.cloudMode && this.tenant !== value) { this.tenant = value; this.exchange = ''; } })
        TextInput({ placeholder: 'ID projektu', text: this.projectId })
          .enabled(!this.saving && !this.cloudMode)
          .onChange((value: string) => { if (!this.saving && !this.cloudMode && this.projectId !== value) { this.projectId = value; this.exchange = ''; } })

        if (this.codeView) {
          Text('Code: edytuj kanoniczny JSON DAG')
            .fontColor('#F4F1EA')
          TextArea({ text: this.source, placeholder: 'JSON DAG' })
            .height(320)
            .enabled(!this.saving && !this.cloudMode)
            .onChange((value: string) => { if (!this.saving && !this.cloudMode) this.source = value; })
        } else {
          Text('NoCode: typowany graf i bezpieczna transformacja')
            .fontColor('#F4F1EA')
          TextInput({ placeholder: 'ID nowego Transform', text: this.nextNodeId })
            .onChange((value: string) => { this.nextNodeId = value; })
          Button('Dodaj Transform przed wyjściem')
            .enabled(!this.saving && !this.cloudMode)
            .onClick(() => { this.updateFromNoCode('transform'); })
          TextInput({ placeholder: 'Nazwa narzędzia', text: this.toolName })
            .onChange((value: string) => { this.toolName = value; })
          TextInput({ placeholder: 'Koszt Tool w jednostkach minimalnych', text: this.toolCost })
            .onChange((value: string) => { this.toolCost = value; })
          Button('Dodaj Tool przed wyjściem')
            .enabled(!this.saving && !this.cloudMode)
            .onClick(() => { this.updateFromNoCode('tool'); })
          Button('Zmień nazwę i koszt Tool')
            .enabled(!this.saving && !this.cloudMode)
            .onClick(() => { this.updateFromNoCode('update-tool'); })
          TextInput({ placeholder: 'Nowe ID węzła', text: this.renameTo })
            .onChange((value: string) => { this.renameTo = value; })
          Button('Zmień ID wskazanego węzła')
            .enabled(!this.saving && !this.cloudMode)
            .onClick(() => { this.updateFromNoCode('rename'); })
          Button('Usuń wskazany Transform / Tool')
            .enabled(!this.saving && !this.cloudMode)
            .onClick(() => { this.updateFromNoCode('remove'); })
          Text('Weryfikacja DAG: kliknij „Sprawdź plan”.')
            .fontColor('#9499A3')
          Text(this.source)
            .fontSize(11)
            .fontColor('#9499A3')
        }

        Row({ space: 8 }) {
          Button('Nowy').enabled(!this.saving && !this.cloudMode).onClick(() => { this.create(); })
          Button('Sprawdź plan').enabled(!this.saving && !this.cloudMode).onClick(() => { this.validate(); })
        }
        Row({ space: 8 }) {
          Button('Zapisz lokalnie').enabled(!this.saving && !this.cloudMode).onClick(() => { this.save(); })
          Button('Otwórz').enabled(!this.saving && !this.cloudMode).onClick(() => { this.load(); })
        }

        if (!this.cloudMode) {
          Text('Wymiana projektu JSON (bez podpisu, bez automatycznego zapisu)')
            .fontColor('#F4F1EA')
          TextArea({ text: this.exchange, placeholder: 'Wklej JSON projektu albo wybierz Eksport JSON' })
            .height(220)
            .maxLength(131072)
            .enabled(!this.saving && !this.cloudMode)
            .onChange((value: string) => { if (!this.saving && !this.cloudMode) this.exchange = value; })
          Row({ space: 8 }) {
            Button('Eksport JSON').enabled(!this.saving && !this.cloudMode).onClick(() => { this.exportProject(); })
            Button('Import JSON').enabled(!this.saving && !this.cloudMode).onClick(() => { this.importProject(); })
          }
          Text('Import zastępuje projekt w pamięci; nie zmienia zapisanego projektu.')
            .fontSize(12)
            .fontColor('#9499A3')
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
