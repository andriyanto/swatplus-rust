## Lampiran A  Data hujan satelit GSMaP: alasan, akses, dan rencana

Lampiran ini mencatat kenapa proyek memerlukan data hujan satelit GSMaP (*Global Satellite Mapping of Precipitation*) dari JAXA, kapan data itu diperlukan, cara mendapatkan aksesnya, dan rencana pemakaiannya di RustGIS dan engine. Lampiran ini bagian dari rencana pengembangan (`docs/roadmap.md`), bukan bagian engine 1.0. Ditulis 8 November 2026.

### A.1  Masalah: data hujan di DAS Indonesia

Hasil SWAT+ dan model banjir paling ditentukan oleh hujan yang dimasukkan. Di Indonesia, data hujan punya tiga kendala:

- **Pos hujan jarang dan tidak merata.** DAS sebesar Batang Hari (±85.000 km²) hanya punya sedikit pos dengan data lengkap, padahal hujan tropis sangat lokal: hujan lebat di satu lembah bisa tidak tercatat di pos terdekat.
- **Data pos sering bolong dan terlambat.** Banyak pos masih manual, datanya terkumpul berminggu-minggu kemudian.
- **Prakiraan banjir memerlukan data saat itu juga**, bukan bulan berikutnya.

### A.2  Apa itu GSMaP

GSMaP memperkirakan hujan dari satelit: citra inframerah satelit geostasioner (Himawari untuk wilayah Indonesia) digabung dengan satelit gelombang mikro. Ciri-cirinya:

| Ciri | Nilai |
| --- | --- |
| Cakupan | 60° LU – 60° LS, seluruh Indonesia tanpa celah |
| Resolusi ruang | 0,1° (±11 km) |
| Resolusi waktu | per jam; GSMaP_NOW diperbarui setiap 30 menit |
| Satuan | mm/jam (rata-rata satu jam) |
| Produk | GSMaP_NOW (hampir waktu nyata), GSMaP_NRT (beberapa jam kemudian), GSMaP_MVK (standar/reanalisis), dan versi *Gauge* yang dikoreksi dengan data pos hujan |
| Biaya | gratis, dengan registrasi |

Format: produk lama (sampai Desember 2021) berupa biner *float* 4 byte *little-endian*, grid 3600 × 1200, piksel pertama berpusat di 59,95° LU, 0,05° BT, baris dari utara ke selatan, bujur bertambah ke timur dari 0°, nilai negatif berarti tidak ada data, berkas terkompresi gzip. Produk sekarang (v8 sejak Desember 2021, v9 sejak Oktober 2026) berformat NetCDF dan CSV per wilayah. Rincian format terbaru harus dibaca dari dokumen *Data Format Description* di server JAXA, karena belum bisa diperiksa langsung saat lampiran ini ditulis.

### A.3  Kegunaan di proyek ini

| Kegunaan | Tahap roadmap | Produk yang dipakai |
| --- | --- | --- |
| Masukan prakiraan banjir: hujan yang baru turun dimasukkan ke model untuk memprakirakan debit dan genangan beberapa jam sampai hari ke depan | H6 (F7) | GSMaP_NOW, Gauge_NOW |
| Peta hujan waktu nyata wilayah Indonesia di RustGIS, mode ilmiah (kelas intensitas BMKG, akumulasi 3/6/24 jam) dan mode realistis | V1 (F1 mode ilmiah, F6 mode realistis) | GSMaP_NOW |
| Hujan rata-rata per subDAS sebagai masukan SWAT+ (alat `zonal_means` RustGIS) | F1, F7 | semua |
| Pelengkap pos hujan untuk kalibrasi di subDAS tanpa pos | riset MPhil (opsional) | arsip GSMaP_MVK / Gauge |

### A.4  Kapan belum diperlukan

- **Riset MPhil** (kalibrasi berbasis TMA) bisa memakai data hujan yang sudah ada di model (CHIRPS atau pos hujan). GSMaP tidak menghambat riset itu.
- **Fase F2–F5** (simpan-muat keadaan, `lu_change`, hidrolika 1D dan 2D, rekayasa terain) tidak memerlukannya.

Karena itu pendaftaran akun dilakukan lebih awal (prosesnya beberapa hari), tetapi pekerjaan pembaca GSMaP tidak didahulukan.

### A.5  Keterbatasan

- GSMaP adalah **perkiraan**, bukan pengukuran. Umumnya meleset pada hujan sangat lebat yang singkat dan di daerah pegunungan.
- Sebelum dipakai untuk prakiraan, GSMaP harus **dibandingkan dengan data pos hujan** di DAS yang ditinjau (bias, korelasi, deteksi hujan lebat), dan sebaiknya dikoreksi; versi *Gauge* membantu.
- Bingkai animasi yang disisipkan di antara dua data (lihat V1) hanya tampilan, tidak dipakai sebagai masukan model.

### A.6  Cara mendapatkan akses

1. **Registrasi** di <https://sharaku.eorc.jaxa.jp/GSMaP/registration.html>: nama, email, afiliasi, tujuan pemakaian (misalnya *research on flood modelling and hydrological model (SWAT+) input for Indonesian river basins*), dan persetujuan ketentuan pemakaian (*Terms of Use*).
2. **Tunggu email JAXA** berisi *username* dan *password* untuk server unduhan `hokusai.eorc.jaxa.jp` (protokol SFTP, port 22). Password tidak dibagikan ke siapa pun, termasuk asisten AI.
3. **Unduh dari komputer sendiri** dengan WinSCP atau FileZilla (SFTP, host `hokusai.eorc.jaxa.jp`, port 22). Lingkungan kerja Claude tidak bisa menjangkau server ini.
4. **Contoh untuk pengembangan pembaca**: dua sampai tiga berkas GSMaP_NOW per jam yang berurutan, satu berkas Gauge_NOW bila ada, dan dokumen *Data Format Description* dari folder `docs` di server.

### A.7  Ketentuan pemakaian

- Publikasi yang memakai GSMaP mencantumkan sumber datanya (JAXA).
- Penyebaran luas gambar atau data GSMaP (aplikasi publik, situs web) memerlukan pemberitahuan ke JAXA lebih dulu. Karena itu:
  - berkas contoh tidak dimasukkan ke repositori publik; uji otomatis memakai data tiruan berformat sama;
  - RustGIS yang dirilis ke umum tidak membawa data GSMaP; setiap pengguna memakai akun JAXA masing-masing, kecuali sudah ada izin JAXA.

### A.8  Rencana pengerjaan

| Langkah | Isi | Prasyarat |
| --- | --- | --- |
| 1 | Pembaca format biner lama (terdokumentasi) dengan uji data tiruan | — |
| 2 | Pembaca format NetCDF/CSV terbaru, dicocokkan dengan berkas contoh | akun JAXA, berkas contoh |
| 3 | Peta hujan mode ilmiah di RustGIS: kelas intensitas BMKG, akumulasi 3/6/24 jam | langkah 1–2 |
| 4 | Hujan rata-rata per subDAS dan penulisan berkas `pcp` SWAT+ | langkah 1–2 |
| 5 | Perbandingan GSMaP terhadap pos hujan di DAS uji (Batang Hari, Pessel) | data pos hujan |
| 6 | Masukan waktu nyata ke engine (E4) dan prakiraan (H6) | E1–E4, F7 |

### A.9  Sumber

- JAXA EORC, *GSMaP* (panduan, FAQ produk dan format): <https://sharaku.eorc.jaxa.jp/GSMaP/>
- JAXA, *GSMaP Data Format Description* (produk NRT per jam): <https://ghrc.nsstc.nasa.gov/pub/fieldCampaigns/gpmValidation/ifloods/GSMaP/doc/DataFormatDescription.pdf>
- ITC, contoh akses GSMaP lewat SFTP: <https://filetransfer.itc.nl/pub/52n/ilwis_py/notebooks_V2/HTML_versions/GSMaP_PCP_SPI_Monthly.html>
