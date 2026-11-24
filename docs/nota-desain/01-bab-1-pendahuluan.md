## Bab 1  Pendahuluan

Nota ini menjelaskan engine SWAT+ versi Rust (`swatplus-rust`) fungsi demi fungsi, dalam keadaannya pada versi 1.2.0: apa tugas setiap fungsi, subrutin SWAT+ mana yang ditirunya, persamaan yang dihitungnya, dan kebiasaan SWAT+ yang sengaja dipertahankan. Engine ini adalah terjemahan SWAT+ Fortran commit `6f51536` (16 Oktober 2026) yang menghasilkan tabel keluaran yang sama bit demi bit.

Engine 1.0.0 adalah dasarnya. Perbaikan 1.0.1 sampai 1.0.3, tambahan 1.1 (simulasi per hari, kalibrasi di memori, banyak run sekaligus, mode paralel, hidraulika saluran) dan 1.2 (kalibrasi di tengah run dan pemanasan bersama, untuk studi spin-up) dimasukkan ke bab yang membahas fungsi bersangkutan, dengan keterangan versi di tempatnya. Lampiran B mencatat jalur M (riset kalibrasi berbasis tinggi muka air) langkah demi langkah, beserta catatan kerjanya.

### 1.1  Tujuan nota

- Menjadi rujukan teknis bagi siapa pun yang membaca, memperbaiki, atau memperluas engine.
- Menjelaskan hubungan setiap fungsi Rust dengan kode SWAT+ aslinya, sehingga perbedaan atau bug bisa dilacak ke sumbernya.
- Mencatat keputusan desain: kenapa hasil harus identik, kenapa kebiasaan SWAT+ dipertahankan, kenapa engine dipisah dari RustGIS.
- Menyediakan glosarium istilah dan daftar pustaka dari persamaan yang dipakai.

### 1.2  Ruang lingkup engine

Engine menjalankan proyek SWAT+ dari lahan sampai outlet:

- **HRU**: cuaca, neraca air, tanaman, nutrien, karbon (model asli dan Century), erosi, manajemen berdasarkan tanggal, heat unit, dan tabel keputusan, sawah, lahan basah, tile drain dan struktur lain, septik, kawasan urban.
- **Routing**: routing lanskap antar HRU, unit routing, akuifer, saluran SWAT-DEG, reservoir, dan sumber titik (recall).
- **Langkah waktu**: harian, atau subharian dengan Green-Ampt atau curve number dengan hidrograf satuan.
- **Keluaran**: tabel teks SWAT+, tabel yang sama dalam Parquet, arsip tar, dan progres JSON-line.
- **Platform**: program native (Linux dan Windows untuk x86-64, dan sejak 1.0.2 juga ARM64; macOS Apple silicon dan Intel) dan modul WebAssembly untuk browser.
- **Simulasi per hari (1.1)**: `Simulation` berjalan satu hari setiap panggilan, sehingga pemanggil dapat berhenti di antara dua hari, memeriksa keadaan, menyalinnya (`checkpoint`), mengubah parameter (`calibrate`), lalu melanjutkan (Bab 5).
- **Kalibrasi (1.1)**: perubahan parameter dari memori, bukan dari berkas (`Calibration`), dan banyak run sekaligus dengan perintah `batch` (Bab 3, 5, 14, 16; Lampiran B).
- **Mode paralel (1.1)**: dengan `--fix-swat --threads N`, HRU dan routing berjalan di beberapa thread dengan tabel yang sama untuk berapa pun jumlah thread (Bab 5, 12).
- **Hidraulika saluran (1.1)**: tabel tambahan `channel_hyd_day.txt` dari kurva rating SWAT-DEG dan rumus Manning; bukan tabel SWAT+ (Bab 12, 15).

Fitur yang belum di-port ditolak dengan namanya: `run` menyebut alasannya dan meneruskan air bagian itu, atau gagal bila `--strict` dipakai. Engine tidak pernah menebak.

### 1.3  Dasar validasi

Tabel keluaran dibandingkan dengan SWAT+ Fortran pada dua contoh SWAT+ (Osu\_1hru, Ames\_sub1), dua proyek nyata dari Sumatera Barat (Batanghari, 1.858 HRU; Pessel, 3.966 HRU), dan 33 varian yang menguji fitur tertentu. CI menjalankan kedua engine pada setiap push dan membandingkan setiap tabel. Target keputusan K12: beda relatif paling banyak 1e-4 atau satu satuan pada desimal terakhir yang dicetak. Pada kenyataannya hasil identik bit demi bit; sejak 1.0.3 itu berlaku juga untuk tabel recall dan titik sumber, yang sebelumnya berbeda paling banyak 6e-8 karena bentuk tulisan angkanya (Bab 15). Setiap tambahan 1.1 diperiksa dengan cara yang sama: run setia (tanpa `--fix-swat`) memberi tabel yang identik byte demi byte dengan 1.0.3 pada ke-37 skenario. Rinciannya di Bab 17.

### 1.4  Konvensi penulisan

| Konvensi | Arti |
| --- | --- |
| `nama_fungsi` | Fungsi atau method Rust, ditulis seperti di kode |
| Padanan SWAT+ | Subrutin Fortran (`nama.f90`) yang ditiru fungsi itu |
| Kebiasaan SWAT+ (quirk) | Perilaku SWAT+ yang mengejutkan atau keliru tetapi dipertahankan agar hasil identik |
| `--fix-swat` | Mode koreksi: kebiasaan yang jelas bug diperbaiki, hasil tidak lagi sama dengan SWAT+ |
| f32 / f64 | Presisi tunggal / ganda. SWAT+ memakai `real` (f32) untuk hampir semua variabel, sehingga engine juga memakai f32 |
| mm, kg/ha, m3, ha, °C, MJ/m2 | Satuan seperti di SWAT+ |

Persamaan ditulis dalam notasi matematika, bukan salinan kode Fortran. SWAT+ berlisensi LGPL-2.1, dan engine ini juga LGPL-2.1 karena merupakan terjemahannya.

### 1.5  Cara membaca nota

- Bab 2 memberi gambaran arsitektur dan prinsip desain. Baca ini dulu.
- Bab 3 sampai 16 mengikuti crate dan modul, dari model data, pembaca input, loop simulasi, proses di HRU, routing, sampai keluaran dan program.
- Setiap fungsi punya entri sendiri: lokasi, padanan SWAT+, tujuan, masukan dan keluaran, cara kerja, dan catatan. Fungsi kecil dikumpulkan dalam tabel.
- Bab 17 merangkum validasi, katalog kebiasaan SWAT+, koreksi `--fix-swat`, dan fitur yang ditolak.
- Glosarium dan daftar pustaka ada di akhir. Lampiran A membahas data hujan satelit GSMaP; Lampiran B jalur M (kalibrasi, mode paralel, studi spin-up) dan catatan langkah kerja (B.8); Lampiran C pekerjaan di RustGIS-SwatPlus (model SWAT+ di GIS dan asisten AI), dengan catatan langkah kerja C.6.
