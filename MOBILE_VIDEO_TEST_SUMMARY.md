# Mobil Video Oynatma Test Özeti

Bu belge, `youtube vanced` projesinde Tauri v2 Android + web UI ile video oynatma sorununu çözmek için yapılanları, iPhone ve Android testlerini ve çıkan sonuçları özetler. Gelecekteki LLM/insan katkıcıların bağlamı hızla anlaması için yazılmıştır.

---

## 1. Sorunun Tanımı

- Uygulamanın web UI'si varsayılan olarak `MockClient` kullanıyordu.
- Mock video URL'leri (`commondatastorage.googleapis.com`) mobil/WebView ortamlarda **ORB (Opaque Response Blocking)** nedeniyle engelleniyordu.
- Android WebView'da video yüklenmiyordu; kullanıcı sadece boş bir oynatıcı/skeleton görüyordu.
- Oynatıcı hata durumlarını kullanıcıya göstermiyor, touch kontroller düzgün çalışmıyordu.

---

## 2. Video Oynatma İçin Yapılan Kod Değişiklikleri

### 2.1 Mock URL'leri Değiştirildi
- **Dosya**: `wt-2-ui/src/lib/mockClient.ts`
- CORS dostu, WebView'da engellenmeyen örnek videolar kullanıldı:
  - `https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4`
  - `https://media.w3.org/2010/05/sintel/trailer.mp4`
  - `https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_1mb.mp4`

### 2.2 VideoPlayer Bileşeni Güçlendirildi
- **Dosya**: `wt-2-ui/src/components/VideoPlayer.tsx`
- Kaynak değiştiğinde `<video>` elementini yeniden mount etmek için `key={currentSrc}` eklendi.
- `waiting`, `canplay`, `playing`, `error` olayları dinlenmeye başlandı.
- Buffering spinner, yükleme hatası mesajı ve **Retry** butonu eklendi.
- Touch davranışı düzeltildi: video alanına dokunulunca play/pause toggle olur, scrub bar ve butonlar ayrı bir `stopPropagation` katmanıyla çalışmaya devam eder.

### 2.3 Watch Sayfası Hata UI'sı Düzeltildi
- **Dosya**: `wt-2-ui/src/routes/Watch.tsx`
- `useQuery` hataları artık sonsuz skeleton yerine `AlertCircle` + hata mesajı + `Retry` butonu ile gösteriliyor.

### 2.4 Tauri Shell UI ile Entegre Edildi
- **Dosyalar**: `wt-1-tauri-shell/src-tauri/tauri.conf.json`, `wt-2-ui/vite.config.ts`
- `tauri.conf.json` build/dev komutları `wt-2-ui` paketini işaret edecek şekilde güncellendi.
- `frontendDist` `../../wt-2-ui/dist` olarak ayarlandı.
- `wt-2-ui/vite.config.ts` içinde `base: './'` eklendi; böylece WebView içinde asset yolları göreceli çözüldü.

---

## 3. Test Ortamları

### 3.1 iPhone Testi (Masaüstü Chromium Mobil Emülasyonu)
- **Ortam**: Chrome/Edge masaüstü tarayıcı, iPhone 13 emülasyonu.
- **Yapılan**: Uygulama yerel geliştirme sunucusunda (`pnpm dev`) açıldı, ana sayfadan bir video kartına tıklandı.
- **Sonuç**: `flower.mp4` başarıyla yüklendi ve oynatıldı. Video görüntüsü ve kontroller çalıştı.
- **Not**: Bu test fiziksel bir iPhone cihazında değil, tarayıcı emülasyonunda yapıldı.

### 3.2 Android Testi (Android Emülatörü)
- **Ortam**: macOS üzerinde Tauri v2 ile derlenen Android uygulaması, ARM64 Android emülatörü (API 34, Google APIs).
- **SDK hazırlığı**:
  - Eksik lisans sorunu çözüldü: `sdkmanager --sdk_root=/Users/yh/Library/Android/sdk --install "build-tools;35.0.1" "platforms;android-36"`
  - Emülatör ve `system-images;android-34;google_apis;arm64-v8a` kuruldu.
  - ARM64 AVD (`ytb_test`) manuel olarak oluşturulup başlatıldı.
- **Derleme**:
  - Release APK: `pnpm tauri android build --ci --apk`
  - Emülatörde test için debug APK: `pnpm tauri android build --ci --apk --debug -t aarch64`
- **Yükleme & çalıştırma**:
  ```bash
  adb -s emulator-5554 install -r app-universal-debug.apk
  adb -s emulator-5554 shell am start -n com.afstudy20.ytb/com.afstudy20.ytb.MainActivity
  ```
- **Sonuç**: Uygulama açıldı, ana sayfadaki bir video kartına tıklandı, `flower.mp4` Android WebView içinde başarıyla oynatıldı.
- **Logcat**: Uygulamaya özel hata görülmedi; sadece sistem/Google servisleriyle ilgili gürültü vardı.

---

## 4. Çıktılar

- **Release APK**:
  ```
  wt-1-tauri-shell/src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk
  ```
- **Debug APK**:
  ```
  wt-1-tauri-shell/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
  ```
- **Push edilen dal**:
  ```
  afstudy20-gif/fix/android-webview-video-playback
  ```

---

## 5. Bilinen Sınırlamalar

- Release APK **imzalanmamış**; dağıtım için signing gerekiyor.
- iPhone testi fiziksel cihazda değil, tarayıcı emülasyonunda yapıldı.
- `RealClient` henüz çalışmıyor çünkü `wt-3-innertube` sadece bir Rust kütüphanesi; `/videos/:id`, `/streams/:id` gibi endpointleri sunan ayrı bir backend yok.
- Emülatör testi gerçek bir cihaz performansını tam olarak yansıtmayabilir.

---

## 6. Gelecek Adımlar (Opsiyonel)

- Gerçek bir Android cihazda ve fiziksel iPhone'da test yapılması.
- `RealClient` için backend server'ın implementasyonu.
- Release APK'nin imzalanması ve dağıtım kanalları için CI/CD entegrasyonu.
