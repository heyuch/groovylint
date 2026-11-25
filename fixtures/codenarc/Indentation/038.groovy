
  @SuppressWarnings('Indentation')
   class MyClass {
               def processResults() {
                       return list.findAll { it instanceof Date }
       }
}

class MyOtherClass {
       @SuppressWarnings('Indentation')
               def processResults() {
                       return list.findAll { it instanceof Date }
       }
}
        