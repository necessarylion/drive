APP = build/Drive.app
SOURCES = Package.swift Sources Tests Scripts

# The version number of a release build. The release workflow passes the tag, without the "v".
VERSION ?= 0.0.0

.PHONY: app run release test format lint icon

# Build the release binary and wrap it in a .app bundle.
app:
	swift build -c release
	rm -rf $(APP)
	mkdir -p $(APP)/Contents/MacOS $(APP)/Contents/Resources
	cp .build/release/Drive $(APP)/Contents/MacOS/Drive
	cp Resources/Info.plist $(APP)/Contents/Info.plist
	cp Resources/AppIcon.icns $(APP)/Contents/Resources/AppIcon.icns
	codesign --force --sign - $(APP)

run: app
	open $(APP)

# Build the app for both Apple silicon and Intel, stamp the version, and zip it for download.
# The folder of the build output is not the same in every Swift version, so ask the build tool for it.
RELEASE_FLAGS = -c release --arch arm64 --arch x86_64

release:
	swift build $(RELEASE_FLAGS)
	rm -rf $(APP)
	mkdir -p $(APP)/Contents/MacOS $(APP)/Contents/Resources
	cp "$$(swift build $(RELEASE_FLAGS) --show-bin-path)/Drive" $(APP)/Contents/MacOS/Drive
	# ponytail: macOS picks the look of the windows from the SDK version in the binary. "make app" writes 15.0,
	# but the universal build writes the real SDK, so the download got the newer look. Write 15.0 here too.
	# To move to the new look, remove this line and build "make app" with the real SDK version as well.
	vtool -set-build-version macos 15.0 15.0 -replace -output $(APP)/Contents/MacOS/Drive $(APP)/Contents/MacOS/Drive
	cp Resources/Info.plist $(APP)/Contents/Info.plist
	cp Resources/AppIcon.icns $(APP)/Contents/Resources/AppIcon.icns
	plutil -replace CFBundleShortVersionString -string "$(VERSION)" $(APP)/Contents/Info.plist
	plutil -replace CFBundleVersion -string "$(VERSION)" $(APP)/Contents/Info.plist
	codesign --force --sign - $(APP)
	ditto -c -k --keepParent $(APP) build/Drive-$(VERSION).zip

test:
	swift test

format:
	swift format --in-place --recursive $(SOURCES)

lint:
	swift format lint --strict --recursive $(SOURCES)

# Rebuild AppIcon.icns from Resources/Logo.png. That file is exported from the Pencil design.
icon:
	swift Scripts/make-icon.swift Resources/Logo.png build/AppIcon.iconset
	iconutil -c icns build/AppIcon.iconset -o Resources/AppIcon.icns
