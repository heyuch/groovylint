
          def x = String
          def y = x.class
          def z = x.getClass()
          def a = String.class()
          assert codeNarcRunner.sourceAnalyzer.class == AntFileSetSourceAnalyzer
        