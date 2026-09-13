// Setup guides for the calendar services people actually use.
//
// The illustrations are schematics drawn by OpenTempus, not screenshots of
// those services. Menu names and their positions follow the real UI; if a
// provider moves something, fix the label here and link to their own docs.

import type { ShotSpec } from "./Shot";

export interface Step {
  text: string;
  shot?: ShotSpec;
  caption?: string;
}

export interface FillValue {
  label: string;
  value: string;
  /** Render a copy button (useful for fixed server URLs). */
  copy?: boolean;
  note?: string;
}

export interface Guide {
  id: string;
  name: string;
  /** Letter tile; deliberately not the provider's logo. */
  tile: string;
  tone: string;
  /** Which OpenTempus source kind this produces. */
  kind: "ics" | "caldav";
  /** One line for the index card. */
  summary: string;
  /** Can OpenTempus write back into it? */
  twoWay: boolean;
  steps: Step[];
  fill: FillValue[];
  caveats: string[];
  docs: { label: string; url: string }[];
}

// ---------------------------------------------------------------------------
// Reusable final step: the OpenTempus "Add calendar" form.
// ---------------------------------------------------------------------------

function icsShot(url: string, name = "Work"): ShotSpec {
  return {
    url: "",
    bare: true,
    w: 520,
    h: 178,
    shapes: [
      { t: "text", x: 30, y: 24, s: "Add calendar", bold: true, size: 13 },
      { t: "field", x: 30, y: 48, w: 200, s: name, label: "Name" },
      { t: "text", x: 30, y: 96, s: "◉  iCal / ICS address        ○  CalDAV account", size: 11 },
      { t: "field", x: 30, y: 118, w: 474, s: url, label: "iCal / ICS address", mono: true },
      { t: "ring", x: 26, y: 114, w: 482, h: 32, note: "paste it here, then Add & sync", below: true },
    ],
  };
}

function caldavShot(server: string, username: string): ShotSpec {
  return {
    url: "",
    bare: true,
    w: 520,
    h: 252,
    shapes: [
      { t: "text", x: 30, y: 24, s: "Add calendar", bold: true, size: 13 },
      { t: "field", x: 30, y: 48, w: 200, s: "Personal", label: "Name" },
      { t: "text", x: 30, y: 96, s: "○  iCal / ICS address        ◉  CalDAV account", size: 11 },
      { t: "field", x: 30, y: 118, w: 474, s: server, label: "CalDAV server", mono: true },
      { t: "field", x: 30, y: 172, w: 222, s: username, label: "Username" },
      { t: "field", x: 268, y: 172, w: 222, s: "••••••••••••", label: "Password" },
      { t: "btn", x: 30, y: 210, s: "Find calendars" },
      { t: "ring", x: 26, y: 206, w: 114, h: 32, note: "then pick your calendar from the list" },
    ],
  };
}

// ---------------------------------------------------------------------------

export const GUIDES: Guide[] = [
  {
    id: "google",
    name: "Google Calendar",
    tile: "G",
    tone: "#1a73e8",
    kind: "ics",
    twoWay: false,
    summary: "Gmail and Google Workspace. Read-only feed via the secret iCal address.",
    steps: [
      {
        text: "Open Google Calendar on a computer. In the left sidebar under \"My calendars\", hover the calendar you want, click the ⋮ button that appears, and choose \"Settings and sharing\".",
        shot: {
          url: "calendar.google.com",
          h: 236,
          shapes: [
            { t: "panel", x: 12, y: 40, w: 170, h: 182, flat: true },
            { t: "panel", x: 192, y: 40, w: 356, h: 182, flat: true },
            { t: "text", x: 206, y: 62, s: "September 2026", bold: true },
            { t: "text", x: 24, y: 62, s: "My calendars", muted: true, size: 10 },
            { t: "list", x: 24, y: 84, w: 152, items: ["Robbert", "Work", "Family", "Birthdays"], dots: true, active: 1 },
            { t: "kebab", x: 166, y: 101 },
            { t: "menu", x: 180, y: 96, w: 176, items: ["Display this only", "Hide from list", "Settings and sharing"], hi: 2 },
            { t: "cursor", x: 318, y: 152 },
            { t: "ring", x: 183, y: 144, w: 170, h: 22, n: 1 },
          ],
        },
      },
      {
        text: "In the left menu of the settings page, click \"Integrate calendar\".",
        shot: {
          url: "calendar.google.com/calendar/u/0/r/settings/calendar/…",
          h: 236,
          shapes: [
            { t: "panel", x: 12, y: 40, w: 196, h: 182, flat: true },
            { t: "list", x: 24, y: 68, w: 176, items: ["Calendar settings", "Access permissions", "Share with specific people", "Event notifications", "Integrate calendar", "Remove calendar"], active: 4 },
            { t: "text", x: 232, y: 66, s: "Integrate calendar", bold: true, size: 12 },
            { t: "field", x: 232, y: 88, w: 300, s: "robbert@example.com", label: "Calendar ID" },
            { t: "field", x: 232, y: 142, w: 300, s: "…/public/basic.ics", label: "Public address in iCal format", mono: true },
            { t: "field", x: 232, y: 196, w: 300, s: "…/private-8f3c…/basic.ics", label: "Secret address in iCal format", mono: true },
            { t: "ring", x: 20, y: 143, w: 184, h: 22, n: 2 },
            { t: "cursor", x: 146, y: 150 },
          ],
        },
      },
      {
        text: "Copy the value under \"Secret address in iCal format\". It ends in /basic.ics. Do not copy the public address above it: that one only works if the calendar is public.",
        caption: "The secret address is a password in disguise. Anyone holding it can read the whole calendar, so paste it straight into OpenTempus and nowhere else. If it ever leaks, the Reset button next to it issues a new one.",
        shot: {
          url: "",
          bare: true,
          w: 520,
          h: 158,
          shapes: [
            { t: "text", x: 30, y: 24, s: "Integrate calendar", bold: true, size: 13 },
            { t: "field", x: 30, y: 46, w: 460, s: "https://calendar.google.com/calendar/ical/…/public/basic.ics", label: "Public address in iCal format", mono: true, copy: true },
            { t: "field", x: 30, y: 100, w: 460, s: "https://calendar.google.com/calendar/ical/…/private-8f3c…/basic.ics", label: "Secret address in iCal format", mono: true, copy: true },
            { t: "ring", x: 26, y: 96, w: 468, h: 32, n: 3, note: "copy this one", below: true },
          ],
        },
      },
      {
        text: "In OpenTempus: Calendars → Add calendar → \"iCal / ICS address\", paste, set a category such as work, then Add & sync.",
        shot: icsShot("https://calendar.google.com/calendar/ical/…/private-8f3c…/basic.ics"),
      },
    ],
    fill: [
      { label: "Kind", value: "iCal / ICS address" },
      { label: "Address", value: "the secret address ending in /basic.ics" },
    ],
    caveats: [
      "Google caches this feed. New or moved events often show up within minutes but can take a few hours. That delay is on Google's side, not something a shorter sync interval in OpenTempus fixes.",
      "Read-only. Writing \"Busy\" blocks back into Google needs the OAuth connector on the roadmap: Google switched CalDAV off for password-based logins in March 2025, so there is no app-password route left.",
      "A work or school account may hide the secret address if the admin disabled external sharing.",
    ],
    docs: [
      { label: "Google: sync your calendar with other apps", url: "https://support.google.com/calendar/answer/37648" },
      { label: "Google: winding down less secure apps", url: "https://workspaceupdates.googleblog.com/2023/09/winding-down-google-sync-and-less-secure-apps-support.html" },
    ],
  },

  {
    id: "outlook",
    name: "Outlook / Microsoft 365",
    tile: "O",
    tone: "#0f6cbd",
    kind: "ics",
    twoWay: false,
    summary: "outlook.com and work accounts. Read-only feed via a published calendar.",
    steps: [
      {
        text: "In Outlook on the web, switch to Calendar and open Settings (the gear icon). Go to Calendar → Shared calendars.",
        shot: {
          url: "outlook.office.com/calendar",
          h: 250,
          shapes: [
            { t: "panel", x: 12, y: 40, w: 536, h: 22, flat: true },
            { t: "text", x: 24, y: 55, s: "Calendar", bold: true },
            { t: "gear", x: 528, y: 51 },
            { t: "ring", x: 514, y: 37, w: 28, h: 28, n: 1 },
            { t: "panel", x: 80, y: 78, w: 468, h: 160 },
            { t: "text", x: 96, y: 100, s: "Settings", bold: true, size: 12 },
            { t: "list", x: 96, y: 128, w: 110, items: ["General", "Mail", "Calendar", "People"], active: 2 },
            { t: "panel", x: 224, y: 88, w: 1, h: 140 },
            { t: "list", x: 244, y: 128, w: 170, items: ["View", "Events and invitations", "Shared calendars"], active: 2 },
            { t: "ring", x: 240, y: 160, w: 178, h: 22, n: 2 },
            { t: "cursor", x: 352, y: 166 },
          ],
        },
      },
      {
        text: "Under \"Publish a calendar\", choose the calendar and how much detail to publish, then click Publish.",
        caption: "Publish \"Can view all details\" unless you have a reason not to. OpenTempus can hide anything it receives, but it can never show what Outlook did not publish: with a busy-only feed, even friends you fully trust will only ever see blocks.",
        shot: {
          url: "outlook.office.com/calendar",
          h: 212,
          shapes: [
            { t: "text", x: 24, y: 52, s: "Shared calendars", bold: true, size: 12 },
            { t: "text", x: 24, y: 82, s: "Publish a calendar", bold: true },
            { t: "select", x: 24, y: 106, w: 190, s: "Calendar", label: "Calendar" },
            { t: "select", x: 236, y: 106, w: 250, s: "Can view all details", label: "Permissions" },
            { t: "ring", x: 232, y: 102, w: 258, h: 32, n: 3, note: "pick the detail level", below: true },
            { t: "btn", x: 24, y: 166, s: "Publish", primary: true },
          ],
        },
      },
      {
        text: "Two links appear. Copy the ICS one, not the HTML one. The HTML link opens a web page; only the ICS link can be subscribed to.",
        shot: {
          url: "",
          bare: true,
          w: 520,
          h: 142,
          shapes: [
            { t: "text", x: 30, y: 24, s: "Publish a calendar", bold: true, size: 13 },
            { t: "text", x: 30, y: 58, s: "HTML", muted: true, size: 10 },
            { t: "field", x: 86, y: 42, w: 418, s: "https://outlook.office.com/owa/calendar/…/reachcalendar.html", mono: true, copy: true },
            { t: "text", x: 30, y: 102, s: "ICS", muted: true, size: 10 },
            { t: "field", x: 86, y: 86, w: 418, s: "https://outlook.office.com/owa/calendar/…/calendar.ics", mono: true, copy: true },
            { t: "ring", x: 82, y: 82, w: 426, h: 32, n: 4, note: "this is the one", below: true },
          ],
        },
      },
      {
        text: "In OpenTempus: Calendars → Add calendar → \"iCal / ICS address\", paste, set a category such as work, then Add & sync.",
        shot: icsShot("https://outlook.office.com/owa/calendar/…/calendar.ics"),
      },
    ],
    fill: [
      { label: "Kind", value: "iCal / ICS address" },
      { label: "Address", value: "the published link ending in .ics" },
    ],
    caveats: [
      "A published calendar is readable by anyone who has the link, at the detail level you chose. Treat it as a secret. \"Unpublish\" on the same screen revokes it immediately.",
      "Work or school accounts: if \"Publish a calendar\" is missing or greyed out, your tenant admin has disabled calendar publishing. There is no way around it from your side.",
      "Read-only. Outlook.com dropped CalDAV years ago, so writing back needs the Microsoft Graph connector on the roadmap.",
    ],
    docs: [{ label: "Microsoft: share your calendar in Outlook.com", url: "https://support.microsoft.com/en-us/office/share-your-calendar-in-outlook-com-0fc1cb48-569d-4d1e-ac20-5a9b3f5e6ff2" }],
  },

  {
    id: "proton",
    name: "Proton Calendar",
    tile: "P",
    tone: "#6d4aff",
    kind: "ics",
    twoWay: false,
    summary: "Read-only feed via a share link. Proton has no CalDAV, so it cannot receive events.",
    steps: [
      {
        text: "Open Proton Calendar in a browser. In the sidebar, hover the calendar, click the ⋮ button and choose Share. (Settings → Calendars → Share gets you to the same place.)",
        shot: {
          url: "calendar.proton.me",
          h: 236,
          shapes: [
            { t: "panel", x: 12, y: 40, w: 170, h: 182, flat: true },
            { t: "panel", x: 192, y: 40, w: 356, h: 182, flat: true },
            { t: "text", x: 206, y: 62, s: "September 2026", bold: true },
            { t: "text", x: 24, y: 62, s: "My calendars", muted: true, size: 10 },
            { t: "list", x: 24, y: 84, w: 152, items: ["My calendar", "Sport", "Family"], dots: true, active: 0 },
            { t: "kebab", x: 166, y: 79 },
            { t: "menu", x: 180, y: 74, w: 150, items: ["Edit calendar", "Share", "Export ICS", "Delete"], hi: 1 },
            { t: "cursor", x: 236, y: 108 },
            { t: "ring", x: 183, y: 100, w: 144, h: 22, n: 1 },
          ],
        },
      },
      {
        text: "Pick \"Share with anyone\", choose the access level and create the link. \"Full view\" shares event details; \"Limited view\" shares only busy blocks.",
        shot: {
          url: "calendar.proton.me",
          h: 236,
          shapes: [
            { t: "panel", x: 90, y: 55, w: 380, h: 166 },
            { t: "text", x: 108, y: 82, s: "Share calendar", bold: true, size: 12 },
            { t: "text", x: 108, y: 104, s: "Share with anyone", muted: true },
            { t: "select", x: 108, y: 136, w: 160, s: "Full view", label: "Access" },
            { t: "btn", x: 286, y: 136, s: "Create link", primary: true },
            { t: "ring", x: 104, y: 132, w: 168, h: 32, n: 2, note: "Full view keeps titles and locations", below: true },
          ],
        },
      },
      {
        text: "Copy the link that appears. It points at calendar.proton.me and ends in calendar.ics with a key in the query string.",
        caption: "That key is what decrypts the calendar, so the URL is the secret. Proton allows up to five links per calendar and you can revoke any of them from this screen.",
        shot: {
          url: "",
          bare: true,
          w: 520,
          h: 102,
          shapes: [
            { t: "text", x: 30, y: 24, s: "Share link", bold: true, size: 13 },
            { t: "field", x: 30, y: 40, w: 460, s: "https://calendar.proton.me/api/calendar/v1/url/…/calendar.ics?CacheKey=…", mono: true, copy: true },
            { t: "ring", x: 26, y: 36, w: 468, h: 32, n: 3, note: "copy", below: true },
          ],
        },
      },
      {
        text: "In OpenTempus: Calendars → Add calendar → \"iCal / ICS address\", paste, set a category such as personal, then Add & sync.",
        shot: icsShot("https://calendar.proton.me/api/calendar/v1/url/…/calendar.ics?CacheKey=…", "Personal"),
      },
    ],
    fill: [
      { label: "Kind", value: "iCal / ICS address" },
      { label: "Address", value: "the Proton share link ending in calendar.ics" },
    ],
    caveats: [
      "Proton does not offer CalDAV, by design: their calendars are end-to-end encrypted. A Proton calendar can be a source in OpenTempus but never a sync target.",
      "If OpenTempus cannot fetch the link, open it in a browser first. It should download an .ics file. If the browser downloads nothing, recreate the link.",
      "A \"Limited view\" link carries no titles, so nothing downstream can ever show them. Use Full view here and let OpenTempus decide who sees what.",
    ],
    docs: [
      { label: "Proton: share a calendar via link", url: "https://proton.me/support/share-calendar-via-link" },
      { label: "Proton: calendar sharing and subscription", url: "https://proton.me/support/calendar/using-calendar/calendar-sharing-subscription" },
    ],
  },

  {
    id: "mailbox",
    name: "mailbox.org",
    tile: "M",
    tone: "#0a7d5a",
    kind: "caldav",
    twoWay: true,
    summary: "Full CalDAV. Works as a source and as a sync target, with an application password.",
    steps: [
      {
        text: "Sign in to mailbox.org and open All settings → Security → Application passwords.",
        shot: {
          url: "office.mailbox.org",
          h: 250,
          shapes: [
            { t: "panel", x: 12, y: 40, w: 156, h: 196, flat: true },
            { t: "list", x: 24, y: 68, w: 136, items: ["Basic settings", "Security", "Mail", "Calendar", "Drive"], active: 1 },
            { t: "ring", x: 20, y: 77, w: 144, h: 22, n: 1 },
            { t: "cursor", x: 86, y: 84 },
            { t: "text", x: 192, y: 66, s: "Application passwords", bold: true, size: 12 },
            { t: "text", x: 192, y: 90, s: "DAVx5 · created 12 Mar 2026", muted: true, size: 10 },
            { t: "select", x: 192, y: 118, w: 176, s: "CalDAV/CardDAV", label: "Application" },
            { t: "field", x: 384, y: 118, w: 152, s: "OpenTempus", label: "Label" },
            { t: "btn", x: 192, y: 168, s: "Add new password", primary: true },
            { t: "ring", x: 188, y: 164, w: 140, h: 32, n: 2 },
          ],
        },
      },
      {
        text: "Choose the CalDAV/CardDAV application type, label it OpenTempus, and add the password. Copy it right away: mailbox.org shows it once and never again.",
        shot: {
          url: "",
          bare: true,
          w: 460,
          h: 100,
          shapes: [
            { t: "text", x: 30, y: 24, s: "Password created", bold: true, size: 13 },
            { t: "field", x: 30, y: 40, w: 396, s: "k7fq-2xza-9mdp-4vlt", mono: true, copy: true },
            { t: "ring", x: 26, y: 36, w: 404, h: 32, n: 3, note: "copy it now", below: true },
          ],
        },
      },
      {
        text: "In OpenTempus: Calendars → Add calendar → \"CalDAV account\". Server https://dav.mailbox.org/caldav/, username is your full mailbox.org address, password is the application password. Click Find calendars and pick one.",
        shot: caldavShot("https://dav.mailbox.org/caldav/", "you@mailbox.org"),
      },
    ],
    fill: [
      { label: "Kind", value: "CalDAV account" },
      { label: "Server", value: "https://dav.mailbox.org/caldav/", copy: true, note: "if nothing is found, try https://dav.mailbox.org/" },
      { label: "Username", value: "your full mailbox.org address" },
      { label: "Password", value: "the application password, never your main password" },
    ],
    caveats: [
      "With two-factor authentication enabled, your main password will not work for CalDAV at all. The application password is the only way in.",
      "Real CalDAV, so the same account can also be a sync target: OpenTempus can write \"Busy\" blocks from your other calendars into it.",
    ],
    docs: [
      { label: "mailbox.org: application passwords", url: "https://kb.mailbox.org/en/private/security-and-privacy/application-passwords-for-external-programs/" },
      { label: "mailbox.org: CalDAV and CardDAV", url: "https://kb.mailbox.org/en/private/addressbook-articles/caldav-cardav-review-article/" },
    ],
  },

  {
    id: "icloud",
    name: "Apple iCloud",
    tile: "A",
    tone: "#555a66",
    kind: "caldav",
    twoWay: true,
    summary: "Full CalDAV with an app-specific password. Source and sync target.",
    steps: [
      {
        text: "Go to account.apple.com and sign in. Open Sign-In and Security → App-Specific Passwords, generate one and label it OpenTempus. Copy it; it is shown once.",
        shot: {
          url: "account.apple.com",
          h: 196,
          shapes: [
            { t: "panel", x: 12, y: 40, w: 170, h: 142, flat: true },
            { t: "list", x: 24, y: 68, w: 150, items: ["Personal Information", "Sign-In and Security", "Payment", "Devices"], active: 1 },
            { t: "ring", x: 20, y: 77, w: 158, h: 22, n: 1 },
            { t: "text", x: 206, y: 66, s: "App-Specific Passwords", bold: true, size: 12 },
            { t: "field", x: 206, y: 88, w: 200, s: "OpenTempus", label: "Label" },
            { t: "btn", x: 206, y: 138, s: "Create", primary: true },
            { t: "ring", x: 202, y: 134, w: 70, h: 32, n: 2 },
          ],
        },
      },
      {
        text: "In OpenTempus: Calendars → Add calendar → \"CalDAV account\". Server https://caldav.icloud.com/, username is your Apple Account email, password is the app-specific password.",
        shot: caldavShot("https://caldav.icloud.com/", "you@icloud.com"),
      },
    ],
    fill: [
      { label: "Kind", value: "CalDAV account" },
      { label: "Server", value: "https://caldav.icloud.com/", copy: true },
      { label: "Username", value: "your Apple Account email address" },
      { label: "Password", value: "the app-specific password" },
    ],
    caveats: [
      "App-specific passwords require two-factor authentication on the Apple Account.",
      "Shared iCloud calendars owned by someone else may not appear. Only calendars in your own account are listed.",
    ],
    docs: [{ label: "Apple: app-specific passwords", url: "https://support.apple.com/en-us/102654" }],
  },

  {
    id: "fastmail",
    name: "Fastmail",
    tile: "F",
    tone: "#2b6cb0",
    kind: "caldav",
    twoWay: true,
    summary: "Full CalDAV with an app password. Source and sync target.",
    steps: [
      {
        text: "In Fastmail, open Settings → Privacy & Security → Connected apps & API tokens → New app password. Give it calendar access and label it OpenTempus.",
      },
      {
        text: "In OpenTempus: Calendars → Add calendar → \"CalDAV account\". Server https://caldav.fastmail.com/, username is your Fastmail address.",
        shot: caldavShot("https://caldav.fastmail.com/", "you@fastmail.com"),
      },
    ],
    fill: [
      { label: "Kind", value: "CalDAV account" },
      { label: "Server", value: "https://caldav.fastmail.com/", copy: true },
      { label: "Username", value: "your Fastmail address" },
      { label: "Password", value: "the app password" },
    ],
    caveats: ["Give the app password calendar access, not mail access, so a leak cannot read your mail."],
    docs: [{ label: "Fastmail: app passwords", url: "https://www.fastmail.help/hc/en-us/articles/360058752854-App-passwords" }],
  },

  {
    id: "nextcloud",
    name: "Nextcloud / ownCloud",
    tile: "N",
    tone: "#0082c9",
    kind: "caldav",
    twoWay: true,
    summary: "Full CalDAV on your own server. Source and sync target.",
    steps: [
      {
        text: "In Nextcloud, open Settings → Security → Devices & sessions, then create a new app password named OpenTempus.",
      },
      {
        text: "In OpenTempus: Calendars → Add calendar → \"CalDAV account\". Server https://your-server/remote.php/dav/, username is your Nextcloud username.",
        shot: caldavShot("https://cloud.example.com/remote.php/dav/", "robbert"),
      },
    ],
    fill: [
      { label: "Kind", value: "CalDAV account" },
      { label: "Server", value: "https://your-server/remote.php/dav/", note: "replace your-server with your own host" },
      { label: "Username", value: "your Nextcloud username" },
      { label: "Password", value: "the app password from Devices & sessions" },
    ],
    caveats: [
      "If your Nextcloud is only reachable inside your network, the OpenTempus server has to be able to reach it too. A browser on your laptop is not enough.",
    ],
    docs: [{ label: "Nextcloud: sync calendars with CalDAV", url: "https://docs.nextcloud.com/server/latest/user_manual/en/groupware/sync_ios.html" }],
  },
];

export function guideById(id: string): Guide | undefined {
  return GUIDES.find((g) => g.id === id);
}
