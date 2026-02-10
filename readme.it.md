# CosmicCaffeineLand - Applet per COSMIC Desktop

## Descrizione
**CosmicCaffeineLand** è un applet per il panel di COSMIC Desktop (System76) che impedisce al sistema di andare in sospensione o spegnere lo schermo, simile all'utility Caffeine per macOS.
Non sono previste modalità a tempo o legate ad un processo. Preferisco integrazioni utili e avere un'applet dall'uso banale invece di avere molte funzioni di nicchia. Un esempio, fondamentale l'integrazione del monitoraggio batteria e dello stato del lid del laptop.

## Caratteristiche Principali

### ☕ Funzionalità Core
- **Inibizione sleep/screensaver**: Usa il protocollo Wayland `idle-inhibit-unstable-v1` per impedire la sospensione del sistema
- **Toggle semplice**: Click sull'icona per attivare/disattivare
- **Indicatore visivo**: Icona della tazza ☕ che cambia stato
  - Tazza piena = attivo
  - Tazza vuota = disattivo

### 🔋 Protezione Batteria Intelligente
- **Monitoraggio batteria**: Controlla lo stato della batteria ogni x secondi tramite UPower
- **Disattivazione automatica**: Quando la batteria scende sotto il 10% e il PC non è in carica
- **Blocco riattivazione**: Non permette di riattivare Caffeine finché la batteria non risale o viene collegato il caricatore
- **Notifica batteria bassa**: _"Caffeine è stato disattivato e non può essere riattivato, livello batteria troppo basso"_

### 💻 Rilevamento Chiusura Schermo
- **Monitoraggio lid**: Controlla lo stato del lid del laptop ogni y secondi
- **Rilevamento hardware**: Funziona solo se l'hardware supporta il rilevamento via ACPI (`/proc/acpi/button/lid`)
- **Disattivazione automatica**: Quando lo schermo viene fisicamente chiuso, Caffeine si disattiva automaticamente
- **Notifica chiusura**: _"Caffeine è stato disattivato per aver rilevato la chiusura dello schermo"_
- **Sicurezza**: Opera solo quando può essere certo dello stato del lid tramite hardware

### 🔧 Caratteristiche Tecniche
- **Nessuna dipendenza da systemd**: Completamente indipendente
- **Supporto Wayland nativo**: Usa `zwp_idle_inhibit_manager_v1`
- **Integrazione COSMIC**: Si integra perfettamente con il tema chiaro e scuro del desktop
- **Gestione risorse**: Cleanup automatico delle risorse Wayland alla chiusura

## Setup del Progetto

### Compilazione
```bash
# Crea il progetto
cargo new cosmic-caffeineland-applet
cd cosmic-caffeineland-applet

# Copia il codice in src/main.rs
# Aggiorna Cargo.toml con le dipendenze

# Compila
cargo build --release

# L'eseguibile sarà in target/release/cosmic-caffeineland
```

### Installazione
```bash
# Copia l'eseguibile nella directory appropriata per gli applet COSMIC
# (la posizione esatta dipende dalla configurazione di COSMIC)
sudo cp target/release/cosmic-caffeineland /usr/bin/
```

## Compatibilità
- ✅ COSMIC Desktop 1.0 (stabile)
- ✅ Wayland idle-inhibit protocol (ultima versione stabile)
- ✅ Qualsiasi sistema Linux con UPower per il monitoraggio batteria
- ✅ Laptop con supporto ACPI per il rilevamento lid

## ID Applicazione
`com.system76.CosmicCaffeineLand`

## Dettaglio tecnico importante
**creare e distruggere continuamente le connessioni D-Bus** causava problemi con gli inhibitor, specialmente per lo screensaver.

**Le connessioni persistenti** garantiscono che:
1. La connessione session/system rimane aperta per tutto il tempo in cui caffeine rimane attivo
2. I proxy possono mantenere lo stato correttamente
3. I cookie/file descriptor degli inhibitor rimangono validi

Quindi manteniamo le connessioni come campi della struct principale.
Per essere maggiormente precisi la connessione a dbus vine aperta e chiusa ad ogni ciclo di attivazione/disattivazione.
Le caratteristiche principali sono:

1. **`init()`**: Le connessioni D-Bus iniziano come `None` invece di essere create all'avvio

2. **`update_inhibitor_state()`**: 
   - **All'attivazione** (`is_active = true`): nuove connessioni vengono aperte
   - **Alla disattivazione** (`is_active = false`): le connessioni vengono chiuse impostando a `None`

3. **`Drop`**: Chiusura esplicita delle connessioni anche quando l'applet termina

Questo garantisce che:
- Ad ogni riattivazione vengano stabilite connessioni D-Bus fresche
- Alla disattivazione le connessioni vengano rilasciate
- Non ci siano connessioni persistenti inutilizzate quando caffeine è inattivo

