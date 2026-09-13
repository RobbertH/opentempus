# Connecting your calendars

The same guides are in the app behind the **?** icon in the sidebar, with drawings of each screen. This page is the text version.

Two kinds of connection:

| | What it is | Direction |
| --- | --- | --- |
| **iCal / ICS address** | A URL that serves a calendar file. Every provider has one. | In only |
| **CalDAV account** | Server URL plus credentials. | In, and back out as a sync target |

Only CalDAV can receive events, so "put my personal blockers into my work calendar as Busy" works for CalDAV destinations. Google and Microsoft need OAuth connectors, which are on the [roadmap](ROADMAP.md).

---

## Google Calendar

**Read-only.** Use the secret iCal address.

1. Open Google Calendar on a computer. Hover the calendar under **My calendars**, click the **⋮** button, then **Settings and sharing**.
2. In the left menu, click **Integrate calendar**.
3. Copy **Secret address in iCal format** (it ends in `/basic.ics`). Not the public address above it: that one only works for public calendars.
4. In OpenTempus: Calendars → Add calendar → iCal / ICS address → paste.

Worth knowing:

- The secret address is a password in disguise. Anyone holding it reads the whole calendar. The **Reset** button next to it issues a new one if it leaks.
- Google caches the feed. Changes usually appear within minutes but can take hours. A shorter sync interval in OpenTempus does not help.
- Google switched CalDAV off for password-based logins in March 2025, so there is no two-way route until the OAuth connector lands.

Docs: [sync your calendar with other apps](https://support.google.com/calendar/answer/37648) · [winding down less secure apps](https://workspaceupdates.googleblog.com/2023/09/winding-down-google-sync-and-less-secure-apps-support.html)

---

## Outlook / Microsoft 365

**Read-only.** Publish the calendar.

1. In Outlook on the web, go to Calendar → **Settings** (gear) → **Calendar** → **Shared calendars**.
2. Under **Publish a calendar**, pick the calendar and the detail level, then **Publish**.
3. Copy the **ICS** link, not the HTML one.
4. In OpenTempus: Calendars → Add calendar → iCal / ICS address → paste.

Worth knowing:

- Publish **Can view all details** unless you have a reason not to. OpenTempus can hide anything it receives but can never show what Outlook did not publish, so a busy-only feed means even close friends only ever see blocks.
- Anyone with the published link can read it. **Unpublish** on the same screen revokes it.
- Work or school accounts: if publishing is missing or greyed out, your tenant admin disabled it.
- Outlook.com dropped CalDAV, so two-way needs the Microsoft Graph connector.

Docs: [share your calendar in Outlook.com](https://support.microsoft.com/en-us/office/share-your-calendar-in-outlook-com-0fc1cb48-569d-4d1e-ac20-5a9b3f5e6ff2)

---

## Proton Calendar

**Read-only.** Use a share link.

1. In Proton Calendar, hover the calendar in the sidebar, click **⋮** → **Share** (or Settings → Calendars → Share).
2. Under **Share with anyone**, choose the access level and create the link. **Full view** includes event details; **Limited view** is busy blocks only.
3. Copy the link. It points at `calendar.proton.me` and ends in `calendar.ics` with a key in the query string.
4. In OpenTempus: Calendars → Add calendar → iCal / ICS address → paste.

Worth knowing:

- That key decrypts the calendar, so the URL is the secret. Up to five links per calendar, revocable from the same screen.
- Proton does not offer CalDAV, by design, because their calendars are end-to-end encrypted. Proton can be a source but never a sync target.
- Prefer **Full view** and let OpenTempus decide per audience what to reveal.

Docs: [share a calendar via link](https://proton.me/support/share-calendar-via-link) · [calendar sharing and subscription](https://proton.me/support/calendar/using-calendar/calendar-sharing-subscription)

---

## mailbox.org

**Two-way.** Real CalDAV.

1. Open **All settings → Security → Application passwords**.
2. Add a password, choose the **CalDAV/CardDAV** application type, label it `OpenTempus`. Copy it immediately; it is shown once.
3. In OpenTempus: Calendars → Add calendar → CalDAV account.

| Field | Value |
| --- | --- |
| Server | `https://dav.mailbox.org/caldav/` (if nothing is found, try `https://dav.mailbox.org/`) |
| Username | your full mailbox.org address |
| Password | the application password |

With two-factor authentication on, the main password will not work for CalDAV at all.

Docs: [application passwords](https://kb.mailbox.org/en/private/security-and-privacy/application-passwords-for-external-programs/) · [CalDAV and CardDAV](https://kb.mailbox.org/en/private/addressbook-articles/caldav-cardav-review-article/)

---

## Apple iCloud

**Two-way.** Needs an app-specific password, which needs two-factor authentication on the Apple Account.

1. At [account.apple.com](https://account.apple.com), open **Sign-In and Security → App-Specific Passwords** and create one labelled `OpenTempus`.
2. Server `https://caldav.icloud.com/`, username is your Apple Account email, password is the app-specific password.

---

## Fastmail

**Two-way.**

1. Settings → **Privacy & Security → Connected apps & API tokens → New app password**, with calendar access.
2. Server `https://caldav.fastmail.com/`, username is your Fastmail address.

---

## Nextcloud / ownCloud

**Two-way.**

1. Settings → **Security → Devices & sessions** → create an app password.
2. Server `https://your-server/remote.php/dav/`, username is your Nextcloud username.

The OpenTempus server must be able to reach your Nextcloud. A browser on your laptop being able to is not enough.

---

## Anything else

Any URL serving iCalendar works as a source: Nextcloud share links, Radicale, university timetables, sports club schedules. Any CalDAV server works both ways. `webcal://` links are accepted and rewritten to `https://`.

If a feed fails, the calendar row on the Calendars page shows the error from the last attempt.
