## Bab 1  Pendahuluan

Nota ini, yang sudah diperbarui untuk perbaikan versi 1.0.1, menjelaskan engine SWAT+ versi Rust 1.0.0 (`swatplus-rust`) fungsi demi fungsi: apa tugas setiap fungsi, subrutin SWAT+ mana yang ditirunya, persamaan yang dihitungnya, dan kebiasaan SWAT+ yang sengaja dipertahankan. Engine ini adalah terjemahan SWAT+ Fortran commit `6f51536` (16 Oktober 2026) yang menghasilkan tabel keluaran yang sama bit demi bit.

### 1.1  Tujuan nota

- Menjadi rujukan teknis bagi siapa pun yang membaca, memperbaiki, atau memperluas engine.
- Menjelaskan hubungan setiap fungsi Rust dengan kode SWAT+ aslinya, sehingga perbedaan atau bug bisa dilacak ke sumbernya.
- Mencatat keputusan desain: kenapa hasil harus identik, kenapa kebiasaan SWAT+ dipertahankan, kenapa engine dipisah dari RustGIS.
- Menyediakan glosarium istilah dan daftar pustaka dari persamaan yang dipakai.

### 1.2  Ruang lingkup engine 1.0

Engine 1.0 menjalankan proyek SWAT+ dari lahan sampai outlet:

- **HRU**: cuaca, neraca air, tanaman, nutrien, karbon (model asli dan Century), erosi, manajemen berdasarkan tanggal, heat unit, dan tabel keputusan, sawah, lahan basah, tile drain dan struktur lain, septik, kawasan urban.
- **Routing**: routing lanskap antar HRU, unit routing, akuifer, saluran SWAT-DEG, reservoir, dan sumber titik (recall).
- **Langkah waktu**: harian, atau subharian dengan Green-Ampt atau curve number dengan hidrograf satuan.
- **Keluaran**: tabel teks SWAT+, tabel yang sama dalam Parquet, arsip tar, dan progres JSON-line.
- **Platform**: program native (Linux, macOS, Windows) dan modul WebAssembly untuk browser.

Fitur yang belum di-port ditolak dengan namanya: `run` menyebut alasannya dan meneruskan air bagian itu, atau gagal bila `--strict` dipakai. Engine tidak pernah menebak.

### 1.3  Dasar validasi

Tabel keluaran dibandingkan dengan SWAT+ Fortran pada dua contoh SWAT+ (Osu\_1hru, Ames\_sub1), dua proyek nyata dari Sumatera Barat (Batanghari, 1.858 HRU; Pessel, 3.966 HRU), dan 33 varian yang menguji fitur tertentu. CI menjalankan kedua engine pada setiap push dan membandingkan setiap tabel. Target keputusan K12: beda relatif paling banyak 1e-4 atau satu satuan pada desimal terakhir yang dicetak. Pada kenyataannya hasil identik bit demi bit; rinciannya di Bab 17.

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
- Glosarium dan daftar pustaka ada di akhir.
