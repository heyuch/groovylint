
class MyClass {
    private void execute() {
        try {
            executeWithArgs(args)
        }
        catch(Throwable t) {
            println "ERROR: ${t.message}"
                 t.printStackTrace()        // wrong column
        }
        finally {
            closeResources()
        }
    }
    private void executeOtherOne() {
        try {
      println 999                          // wrong column 
            executeWithArgs(args)
        } catch(Throwable t) {
            t.printStackTrace()
        } finally {
            closeResources()
             notifyUser()                  // wrong column
        }
    }
}
        