@REM the file was created by @redmie
@REM following line command will not echo every line command
@echo off
@REM Allow external definition of anki repository URL
IF "%ANKI_REPO_URL%" == "" GOTO NOURLSET
GOTO END
:NOURLSET
set ANKI_REPO_URL="https://github.com/ankitects/anki"
:END

@REM Set up other variables
set ANKI_TAG=2.1.46
set ANKI_COMMIT=5dab7ed47ec6d17226d2fc0529c32a56e40e5f8a

@REM Clone & patch
echo "Cloning anki from %ANKI_REPO_URL%"
cd %PROJECT_ROOT%
git clone %ANKI_REPO_URL%
echo "Checking out commit %ANKI_COMMIT% and applying patch"
cd anki
git checkout %ANKI_COMMIT%
git apply %ANKI_PATCH_FOLDER%\%ANKI_COMMIT%%ANKI_FILE_SUFFIX%