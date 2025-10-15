      SUBROUTINE CONSTRAINT(THETAS,NTHETAS,SIGMA2,NSIGMAS,OMEGA,NOMEGAS,ITER_NO)
      USE SIZES, ONLY: ISIZE,DPSIZE
      INCLUDE 'TOTAL.INC'
      INTEGER(KIND=ISIZE) NTHETAS,NSIGMAS,NOMEGAS,ITER_NO
      INTEGER I,J,ITER_OLD
      DATA ITER_OLD /-1/
      REAL(KIND=DPSIZE)   :: OMEGA(F_MAXOMEG,F_MAXOMEG),THETAS(F_MAXPTHETA),SIGMA2(F_MAXPTHETA)
      REAL(KIND=DPSIZE), ALLOCATABLE :: OMEGO(:)
      SAVE
!------------------------------------------------------------------------------------
      IF (.NOT.ALLOCATED(OMEGO)) ALLOCATE(OMEGO(F_MAXOMEG)) 
      IF(SAEM_MODE==1 .AND. IMP_MODE==0 .AND. ITS_MODE==0 .AND. ITER_NO<NBURN-200) THEN
      IF(ITER_NO/=ITER_OLD .OR. ITER_NO==0) THEN
! During burn-in phase of SAEM, and when a new iteration occurs (iter_old<>iter_no)
! store the present diagonals of omegas
      ITER_OLD=ITER_NO
      DO I=1,NOMEGAS
      OMEGO(I)=OMEGA(I,I)
      ENDDO
      ENDIF
      DO I=1,NOMEGAS
! Use whatever algorithm needed to "slow down" the reduction of Omega
! The expansion of Omega should be less with each iteration.
      IF(ITER_NO/=0) THEN
      OMEGA(I,I)=OMEGO(I)*(1.0D+00+2.0D+00/ITER_NO)
      ELSE
      OMEGA(I,I)=OMEGO(I)*(1.0D+00+2.0D+00)
      ENDIF
      ENDDO
      ENDIF
      RETURN
!
      END SUBROUTINE CONSTRAINT
