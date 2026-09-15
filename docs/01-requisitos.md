# Unified Google Calendar — Requerimientos

Fecha: 2026-09-14. Estado: cerrado con el usuario. Los cambios se registran al final.

## 1. Problema

El usuario tiene varias cuentas de Google, cada una con obligaciones propias, más eventos personales que no pertenecen a ninguna cuenta. Quiere abrir una sola app de escritorio en Ubuntu y ver todo unificado con la UI de Google Calendar.

## 2. Decisiones de alcance ya tomadas

| Tema | Decisión | Motivo |
|---|---|---|
| Enfoque de UI | App propia que replica Google Calendar pixel a pixel | El admin de Workspace solo permite compartir libre/ocupado hacia afuera, así que embeber la web de Google no unifica. Interceptar el protocolo interno de Google es frágil. |
| Idioma de la UI | Inglés | Facilita comparar contra Google Calendar en inglés. GNOME del usuario está en inglés. |
| Nombre | Unified Google Calendar | Elegido por el usuario. |
| Stack | TypeScript en el frontend. Backend nativo con el menor consumo de RAM posible. | El usuario tiene la RAM limitada. Decisión final del stack en `02-arquitectura.md`. |
| Distribución | Un `.deb`, solo para el usuario. Sin auto actualización. | |

## 3. Cuentas y fuentes de datos

- R3.1 Entre 2 y 3 cuentas de Google, mezcla de Gmail personal y Google Workspace. Todas con lectura y escritura.
- R3.2 Eventos personales locales que no pertenecen a ninguna cuenta de Google. Viven en la base local de la app.
- R3.3 Feriados de Argentina visibles en la grilla.
- R3.4 Sin otras fuentes en la versión 1. No hay Outlook, CalDAV ni `.ics` por URL.
- R3.5 Un evento local se puede mover a una cuenta de Google, y un evento de Google se puede convertir en local.
- R3.6 Riesgo abierto: el admin de una cuenta Workspace puede bloquear apps OAuth de terceros. Se prueba al inicio de la implementación. Si una cuenta queda bloqueada, esa cuenta no entra a la app.

## 4. Funcionalidad de la versión 1

Obligatorio:

- R4.1 Vistas día, semana, mes y agenda, con navegación anterior/siguiente, botón Today y mini calendario en la barra lateral.
- R4.2 Crear, editar y borrar eventos desde el popup rápido y desde el formulario completo. Sin arrastrar ni estirar con el mouse en la versión 1.
- R4.3 Recurrencias completas: diaria, semanal, mensual, anual y personalizada. Editar "this event", "this and following events" y "all events".
- R4.4 Invitados: ver asistentes y su estado, responder Yes / No / Maybe, agregar invitados al crear o editar.
- R4.5 Google Meet: crear eventos con Meet y abrir el link en el navegador del sistema.
- R4.6 Eventos locales con recurrencia y recordatorios, con las mismas reglas que un evento de Google.
- R4.7 Color por calendario, como en Google. La cuenta se identifica en la barra lateral y en el popup del evento.
- R4.8 Detección de choques entre eventos de cuentas distintas, con marca en el chip y aviso en el popup.
- R4.9 Segunda zona horaria visible: Ciudad de México, como columna extra de horas al lado de la principal.
- R4.10 Formato 24 h. La semana empieza el lunes.
- R4.11 Tema claro u oscuro siguiendo el sistema.

Diferido a la versión 2:

- Arrastrar, estirar y crear con click y arrastre.
- Búsqueda de eventos.
- Atajos de teclado de Google Calendar.
- Google Tasks.
- Importar, exportar y suscribirse a `.ics`.
- Fuera de oficina, horario laboral, citas.

## 5. Sincronización

- R5.1 Preferencia fuerte por listeners en lugar de polling. Se usan las notificaciones push de la Calendar API, recibidas en la propia PC por una URL pública provista por Tailscale Funnel.
- R5.2 Al recibir un push, la app pide a Google solo lo que cambió, con sync tokens.
- R5.3 No hace falta modo sin internet. Si se corta la conexión, la app conserva la última UI hasta que vuelva y entonces se pone al día.
- R5.4 Los canales push vencen y la app los renueva sola. Si la PC estuvo dormida, al despertar la app hace una sincronización incremental.
- R5.5 Fallback: si la verificación de dominio de Google no acepta el hostname de Tailscale, la app cae a polling con sync tokens cada 60 s. El diseño debe permitir cambiar de mecanismo sin reescribir la sincronización.

## 6. Integración con Ubuntu y GNOME

- R6.1 Recordatorios como notificaciones nativas de GNOME, con acción para unirse a Meet cuando corresponda.
- R6.2 Ícono en la bandeja de GNOME con el próximo evento. La extensión AppIndicator ya está habilitada en la máquina.
- R6.3 Cerrar la ventana deja la app corriendo en segundo plano, sincronizando y notificando. Se reabre desde la bandeja.
- R6.4 El panel de calendario de GNOME Shell debe mostrar lo mismo que la app. La app escribe sus eventos en Evolution Data Server. Para evitar duplicados, el usuario desactiva Calendar en las cuentas de Google de Online Accounts.
- R6.5 Sin arranque automático al iniciar sesión en la versión 1.

## 7. Seguridad

- R7.1 Los tokens OAuth se guardan en un archivo cifrado propio de la app, no en texto plano.
- R7.2 El endpoint público que recibe los push valida un token secreto por canal y descarta todo lo demás.
- R7.3 Las credenciales OAuth viven en un proyecto de Google Cloud creado por el usuario con su cuenta de Greelow, configurado como app externa. Si el admin de Greelow lo impide, se crea con el Gmail personal.

## 8. Fidelidad visual

- R8.1 La UI replica Google Calendar pixel a pixel: tipografía, tamaños, colores, espaciados, sombras, popups y barra lateral, en tema claro y oscuro.
- R8.2 Cada medida se toma de la web real de Google Calendar con herramientas de desarrollador, no de memoria. El método está en `04-fidelidad-visual.md`.
- R8.3 No se usa el logo de Google. El ícono de la app es propio.

## 9. Rendimiento

- R9.1 RAM lo más baja posible. Objetivo de trabajo: menos de 150 MB con la ventana abierta, menos de 60 MB con la ventana cerrada en segundo plano. Se mide en cada fase.
- R9.2 Rango de datos sincronizados: 1 año hacia atrás y 2 años hacia adelante por calendario.

## 10. Entorno del usuario

- Ubuntu 25.04, GNOME Shell 48, Wayland.
- 14.8 GB de RAM, habitualmente con más de 10 GB en uso.
- WebKitGTK 4.1 versión 2.50.4 instalado. Rust no instalado. Node 20 instalado.
- Evolution Data Server 3.56 con dos cuentas de Google en Online Accounts con Calendar activado.
- Tailscale no instalado.
- Zona horaria principal GMT-3, Argentina.

## Registro de cambios

- 2026-09-14: versión inicial cerrada con el usuario.
- 2026-09-15 (mantenimiento, decisiones del usuario en `99-decisiones.md`): R4.1 suma la vista año. R4.2 cambia: los eventos se crean solo desde el botón "Create event" (formulario modal); el click en la grilla no crea nada y el popup rápido de creación queda sin uso. R4.4 se reduce a ver asistentes y su estado: sin responder Yes/No/Maybe ni agregar invitados desde la app. R8.1 se relaja donde el usuario pidió otra cosa (header, cajón, botón Create, menú de vista, formulario, popup): esas piezas son versiones reducidas construidas con los tokens medidos; el resto sigue pixel a pixel. Se agregan al método de `04` las animaciones, el tooltip, la vista año y el snackbar (componentes 21 a 24).
