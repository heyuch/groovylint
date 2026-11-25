
def queueTests() {
  { stages ->
    queue(queueTestStage())(stages, 'tests', []) {
      'tmp/script.sh'
    }
  }
}
        